use crate::engine::*;
use crate::fee::FeeReserve;
use crate::types::*;
use crate::wasm::{WasmEngine, WasmInstance, WasmInstrumenter, WasmMeteringParams, WasmRuntime};

pub struct ScryptoExecutor<I: WasmInstance> {
    instance: I,
}

impl<I: WasmInstance> Executor<ScryptoValue, ScryptoValue> for ScryptoExecutor<I> {
    fn execute<'s, Y, R>(
        &mut self,
        input: ScryptoValue,
        system_api: &mut Y,
    ) -> Result<ScryptoValue, RuntimeError>
    where
        Y: SystemApi<'s, R>
            + Invokable<ScryptoInvocation, ScryptoValue>
            + Invokable<NativeFunctionInvocation, ScryptoValue>
            + Invokable<NativeMethodInvocation, ScryptoValue>,
        R: FeeReserve,
    {
        let (export_name, return_type, scrypto_actor) = match system_api.get_actor() {
            REActor::Method(
                ResolvedMethod::Scrypto {
                    package_address,
                    blueprint_name,
                    export_name,
                    return_type,
                    ..
                },
                ResolvedReceiver {
                    receiver: RENodeId::Component(component_id),
                    ..
                },
            ) => (
                export_name.to_string(),
                return_type.clone(),
                ScryptoActor::Component(
                    *component_id,
                    package_address.clone(),
                    blueprint_name.clone(),
                ),
            ),
            REActor::Function(ResolvedFunction::Scrypto {
                package_address,
                blueprint_name,
                export_name,
                return_type,
                ..
            }) => (
                export_name.to_string(),
                return_type.clone(),
                ScryptoActor::blueprint(*package_address, blueprint_name.clone()),
            ),

            _ => panic!("Should not get here."),
        };

        let output = {
            let mut runtime: Box<dyn WasmRuntime> =
                Box::new(RadixEngineWasmRuntime::new(scrypto_actor, system_api));
            self.instance
                .invoke_export(&export_name, &input, &mut runtime)
                .map_err(|e| match e {
                    InvokeError::Error(e) => RuntimeError::KernelError(KernelError::WasmError(e)),
                    InvokeError::Downstream(runtime_error) => runtime_error,
                })?
        };

        let rtn = if !return_type.matches(&output.dom) {
            Err(RuntimeError::KernelError(
                KernelError::InvalidScryptoFnOutput,
            ))
        } else {
            Ok(output)
        };

        rtn
    }
}

pub struct ScryptoInterpreter<I: WasmInstance, W: WasmEngine<I>> {
    pub wasm_engine: W,
    /// WASM Instrumenter
    pub wasm_instrumenter: WasmInstrumenter,
    /// WASM metering params
    pub wasm_metering_params: WasmMeteringParams,
    pub phantom: PhantomData<I>,
}

impl<I: WasmInstance, W: WasmEngine<I>> ScryptoInterpreter<I, W> {
    pub fn create_executor(&mut self, code: &[u8]) -> ScryptoExecutor<I> {
        let instrumented_code = self
            .wasm_instrumenter
            .instrument(code, &self.wasm_metering_params);
        let instance = self.wasm_engine.instantiate(instrumented_code);
        ScryptoExecutor { instance }
    }
}
