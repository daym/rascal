macro_rules! semantic_id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub(crate) u32);

        impl $name {
            #[allow(dead_code)]
            pub(crate) fn from_index(index: usize) -> Self {
                Self(u32::try_from(index).expect("semantic arena exceeded u32::MAX entries"))
            }

            #[allow(dead_code)]
            pub(crate) const fn index(self) -> usize {
                self.0 as usize
            }

            pub const fn as_u32(self) -> u32 {
                self.0
            }
        }
    };
}

semantic_id!(DeclId);
semantic_id!(EnvironmentId);
semantic_id!(NameId);
semantic_id!(NodeId);
semantic_id!(ReceiverId);
semantic_id!(RegionId);
semantic_id!(StorageId);
semantic_id!(SymbolId);
semantic_id!(TypeRef);
semantic_id!(TypeSectionId);
semantic_id!(ModuleId);
