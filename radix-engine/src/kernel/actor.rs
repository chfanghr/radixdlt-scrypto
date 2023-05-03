use crate::types::*;
use radix_engine_interface::api::ObjectModuleId;

#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
pub struct InstanceContext {
    pub instance: GlobalAddress,
    pub instance_blueprint: String,
}

#[derive(Debug, PartialEq, Eq, ScryptoSbor)]
pub enum Actor {
    Method {
        global_address: Option<GlobalAddress>,
        node_id: NodeId,
        module_id: ObjectModuleId,
        ident: String,
        object_info: ObjectInfo,
        instance_context: Option<InstanceContext>,
    },
    Function {
        blueprint: Blueprint,
        ident: String,
    },
    VirtualLazyLoad {
        blueprint: Blueprint,
        ident: u8,
    },
}

impl Actor {
    pub fn instance_context(&self) -> Option<InstanceContext> {
        match self {
            Actor::Method {
                instance_context, ..
            } => instance_context.clone(),
            _ => None,
        }
    }

    pub fn blueprint(&self) -> &Blueprint {
        match self {
            Actor::Method {
                object_info: ObjectInfo { blueprint, .. },
                ..
            }
            | Actor::Function { blueprint, .. }
            | Actor::VirtualLazyLoad { blueprint, .. } => blueprint,
        }
    }

    pub fn package_address(&self) -> &PackageAddress {
        let blueprint = match &self {
            Actor::Method {
                object_info: ObjectInfo { blueprint, .. },
                ..
            } => blueprint,
            Actor::Function { blueprint, .. } => blueprint,
            Actor::VirtualLazyLoad { blueprint, .. } => blueprint,
        };

        &blueprint.package_address
    }

    pub fn blueprint_name(&self) -> &str {
        match &self {
            Actor::Method {
                object_info: ObjectInfo { blueprint, .. },
                ..
            }
            | Actor::Function { blueprint, .. }
            | Actor::VirtualLazyLoad { blueprint, .. } => blueprint.blueprint_name.as_str(),
        }
    }

    pub fn method(
        global_address: Option<GlobalAddress>,
        method: MethodIdentifier,
        object_info: ObjectInfo,
        instance_context: Option<InstanceContext>,
    ) -> Self {
        Self::Method {
            global_address,
            node_id: method.0,
            module_id: method.1,
            ident: method.2,
            object_info,
            instance_context,
        }
    }

    pub fn function(ident: FunctionIdentifier) -> Self {
        Self::Function {
            blueprint: ident.0,
            ident: ident.1,
        }
    }

    pub fn virtual_lazy_load(blueprint: Blueprint, ident: u8) -> Self {
        Self::VirtualLazyLoad { blueprint, ident }
    }
}
