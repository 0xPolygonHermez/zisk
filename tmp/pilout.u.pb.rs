extern crate protobuf_upb as __pb;
extern crate std as __std;
pub mod pilout {
#[allow(non_camel_case_types)]
// TODO: Implement support for debug redaction
#[derive(Debug)]
pub struct PilOut {
  inner: ::__pb::__runtime::MessageInner
}

// SAFETY:
// - `PilOut` does not provide shared mutation with its arena.
// - `PilOutMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena that would conflict with
//   field access is impossible.
unsafe impl Sync for PilOut {}

impl ::__pb::Proxied for PilOut {
  type View<'a> = PilOutView<'a>;
  type Mut<'a> = PilOutMut<'a>;
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub struct PilOutView<'a> {
  msg: ::__pb::__internal::RawMessage,
  _phantom: ::__std::marker::PhantomData<&'a ()>,
}

impl<'a> PilOutView<'a> {
  #[doc(hidden)]
  pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
    Self { msg, _phantom: std::marker::PhantomData }
  }
  pub fn r#numProofValues(&self) -> u32 { unsafe {
    pilout_PilOut_numProofValues(self.msg)
  } }

  pub fn r#numPublicValues(&self) -> u32 { unsafe {
    pilout_PilOut_numPublicValues(self.msg)
  } }

}

// SAFETY:
// - `PilOutView` does not perform any mutation.
// - While a `PilOutView` exists, a `PilOutMut` can't exist to mutate
//   the arena that would conflict with field access.
// - `PilOutMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for PilOutView<'_> {}
unsafe impl Send for PilOutView<'_> {}

impl<'a> ::__pb::ViewProxy<'a> for PilOutView<'a> {
  type Proxied = PilOut;

  fn as_view(&self) -> ::__pb::View<'a, PilOut> {
    *self
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, PilOut> where 'a: 'shorter {
    self
  }
}

impl<'a> ::__pb::SettableValue<PilOut> for PilOutView<'a> {
  fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<PilOut>) {
    todo!()
  }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct PilOutMut<'a> {
  inner: ::__pb::__runtime::MutatorMessageRef<'a>,
}

// SAFETY:
// - `PilOutMut` does not perform any shared mutation.
// - `PilOutMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for PilOutMut<'_> {}

impl<'a> ::__pb::MutProxy<'a> for PilOutMut<'a> {
  fn as_mut(&mut self) -> ::__pb::Mut<'_, PilOut> {
    PilOutMut { inner: self.inner }
  }
  fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, PilOut> where 'a : 'shorter { self }
}

impl<'a> ::__pb::ViewProxy<'a> for PilOutMut<'a> {
  type Proxied = PilOut;
  fn as_view(&self) -> ::__pb::View<'_, PilOut> {
    PilOutView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, PilOut> where 'a: 'shorter {
    PilOutView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
}

impl PilOut {
  pub fn new() -> Self {
    let arena = ::__pb::__runtime::Arena::new();
    Self {
      inner: ::__pb::__runtime::MessageInner {
        msg: unsafe { pilout_PilOut_new(arena.raw()) },
        arena,
      }
    }
  }

  pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
    let arena = ::__pb::__runtime::Arena::new();
    let mut len = 0;
    unsafe {
      let data = pilout_PilOut_serialize(self.inner.msg, arena.raw(), &mut len);
      ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
    }
  }
  pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
    let arena = ::__pb::__runtime::Arena::new();
    let msg = unsafe {
      pilout_PilOut_parse(data.as_ptr(), data.len(), arena.raw())
    };

    match msg {
      None => Err(::__pb::ParseError),
      Some(msg) => {
        // This assignment causes self.arena to be dropped and to deallocate
        // any previous message pointed/owned to by self.inner.msg.
        self.inner.arena = arena;
        self.inner.msg = msg;
        Ok(())
      }
    }
  }

  // name: optional string
  pub fn r#name(&self) -> &::__pb::ProtoStr {
    let view = unsafe { pilout_PilOut_name(self.inner.msg).as_ref() };
    // SAFETY: The runtime doesn't require ProtoStr to be UTF-8.
    unsafe { ::__pb::ProtoStr::from_utf8_unchecked(view) }
  }

  pub fn name_opt(&self) -> ::__pb::Optional<&::__pb::ProtoStr> {
    unsafe {
      let view = pilout_PilOut_name(self.inner.msg).as_ref();
      ::__pb::Optional::new(
        // SAFETY: The runtime doesn't require ProtoStr to be UTF-8.
        unsafe { ::__pb::ProtoStr::from_utf8_unchecked(view) }
         ,
        pilout_PilOut_has_name(self.inner.msg)
      )
    }
  }
  pub fn name_mut(&mut self) -> ::__pb::FieldEntry<'_, ::__pb::ProtoStr> {
    static VTABLE: ::__pb::__internal::BytesOptionalMutVTable = unsafe {
      ::__pb::__internal::BytesOptionalMutVTable::new(
        ::__pb::__internal::Private,
        pilout_PilOut_name,
        pilout_PilOut_set_name,
        pilout_PilOut_clear_name,
        b"",
      )
    };
    let out = unsafe {
      let has = pilout_PilOut_has_name(self.inner.msg);
      ::__pb::__internal::new_vtable_field_entry(
        ::__pb::__internal::Private,
        ::__pb::__runtime::MutatorMessageRef::new(
          ::__pb::__internal::Private, &mut self.inner),
        &VTABLE,
        has,
      )
    };
    ::__pb::ProtoStrMut::field_entry_from_bytes(
      ::__pb::__internal::Private, out
    )
  }

  // baseField: optional bytes
  pub fn r#baseField(&self) -> &[u8] {
    let view = unsafe { pilout_PilOut_baseField(self.inner.msg).as_ref() };
    view
  }

  pub fn baseField_mut(&mut self) -> ::__pb::Mut<'_, [u8]> {
    static VTABLE: ::__pb::__internal::BytesMutVTable = unsafe {
      ::__pb::__internal::BytesMutVTable::new(
        ::__pb::__internal::Private,
        pilout_PilOut_baseField,
        pilout_PilOut_set_baseField,
      )
    };
    unsafe {
      <::__pb::Mut<[u8]>>::from_inner(
        ::__pb::__internal::Private,
        ::__pb::__internal::RawVTableMutator::new(
          ::__pb::__internal::Private,
          ::__pb::__runtime::MutatorMessageRef::new(
            ::__pb::__internal::Private, &mut self.inner),
          &VTABLE,
        )
      )
    }
  }

  // subproofs: repeated message pilout.Subproof
  // Unsupported! :(


  // numChallenges: repeated uint32
  // Unsupported! :(


  // numProofValues: optional uint32
  pub fn r#numProofValues(&self) -> u32 {
    unsafe { pilout_PilOut_numProofValues(self.inner.msg) }
  }
  pub fn r#numProofValues_mut(&mut self) -> ::__pb::PrimitiveMut<'_, u32> {
    static VTABLE: ::__pb::__internal::PrimitiveVTable<u32> =
      ::__pb::__internal::PrimitiveVTable::new(
        ::__pb::__internal::Private,
        pilout_PilOut_numProofValues,
        pilout_PilOut_set_numProofValues,
      );

      ::__pb::PrimitiveMut::from_inner(
        ::__pb::__internal::Private,
        unsafe {
          ::__pb::__internal::RawVTableMutator::new(
            ::__pb::__internal::Private,
            ::__pb::__runtime::MutatorMessageRef::new(
              ::__pb::__internal::Private, &mut self.inner
            ),
            &VTABLE,
          )
        },
      )
  }

  // numPublicValues: optional uint32
  pub fn r#numPublicValues(&self) -> u32 {
    unsafe { pilout_PilOut_numPublicValues(self.inner.msg) }
  }
  pub fn r#numPublicValues_mut(&mut self) -> ::__pb::PrimitiveMut<'_, u32> {
    static VTABLE: ::__pb::__internal::PrimitiveVTable<u32> =
      ::__pb::__internal::PrimitiveVTable::new(
        ::__pb::__internal::Private,
        pilout_PilOut_numPublicValues,
        pilout_PilOut_set_numPublicValues,
      );

      ::__pb::PrimitiveMut::from_inner(
        ::__pb::__internal::Private,
        unsafe {
          ::__pb::__internal::RawVTableMutator::new(
            ::__pb::__internal::Private,
            ::__pb::__runtime::MutatorMessageRef::new(
              ::__pb::__internal::Private, &mut self.inner
            ),
            &VTABLE,
          )
        },
      )
  }

  // publicTables: repeated message pilout.PublicTable
  // Unsupported! :(


  // expressions: repeated message pilout.GlobalExpression
  // Unsupported! :(


  // constraints: repeated message pilout.GlobalConstraint
  // Unsupported! :(


  // hints: repeated message pilout.Hint
  // Unsupported! :(


  // symbols: repeated message pilout.Symbol
  // Unsupported! :(



}  // impl PilOut

impl ::__std::ops::Drop for PilOut {
  fn drop(&mut self) {
  }
}

extern "C" {
  fn pilout_PilOut_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
  fn pilout_PilOut_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
  fn pilout_PilOut_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_PilOut_has_name(raw_msg: ::__pb::__internal::RawMessage) -> bool;
  fn pilout_PilOut_name(raw_msg: ::__pb::__internal::RawMessage) -> ::__pb::__internal::PtrAndLen;
  fn pilout_PilOut_set_name(raw_msg: ::__pb::__internal::RawMessage, val: ::__pb::__internal::PtrAndLen);
  fn pilout_PilOut_clear_name(raw_msg: ::__pb::__internal::RawMessage);

  fn pilout_PilOut_baseField(raw_msg: ::__pb::__internal::RawMessage) -> ::__pb::__internal::PtrAndLen;
  fn pilout_PilOut_set_baseField(raw_msg: ::__pb::__internal::RawMessage, val: ::__pb::__internal::PtrAndLen);
  fn pilout_PilOut_clear_baseField(raw_msg: ::__pb::__internal::RawMessage);



  fn pilout_PilOut_numProofValues(raw_msg: ::__pb::__internal::RawMessage) -> u32;
  fn pilout_PilOut_set_numProofValues(raw_msg: ::__pb::__internal::RawMessage, val: u32);
  fn pilout_PilOut_clear_numProofValues(raw_msg: ::__pb::__internal::RawMessage);

  fn pilout_PilOut_numPublicValues(raw_msg: ::__pb::__internal::RawMessage) -> u32;
  fn pilout_PilOut_set_numPublicValues(raw_msg: ::__pb::__internal::RawMessage, val: u32);
  fn pilout_PilOut_clear_numPublicValues(raw_msg: ::__pb::__internal::RawMessage);







}  // extern "C" for PilOut


#[allow(non_camel_case_types)]
// TODO: Implement support for debug redaction
#[derive(Debug)]
pub struct Subproof {
  inner: ::__pb::__runtime::MessageInner
}

// SAFETY:
// - `Subproof` does not provide shared mutation with its arena.
// - `SubproofMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena that would conflict with
//   field access is impossible.
unsafe impl Sync for Subproof {}

impl ::__pb::Proxied for Subproof {
  type View<'a> = SubproofView<'a>;
  type Mut<'a> = SubproofMut<'a>;
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub struct SubproofView<'a> {
  msg: ::__pb::__internal::RawMessage,
  _phantom: ::__std::marker::PhantomData<&'a ()>,
}

impl<'a> SubproofView<'a> {
  #[doc(hidden)]
  pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
    Self { msg, _phantom: std::marker::PhantomData }
  }
  pub fn r#aggregable(&self) -> bool { unsafe {
    pilout_Subproof_aggregable(self.msg)
  } }

}

// SAFETY:
// - `SubproofView` does not perform any mutation.
// - While a `SubproofView` exists, a `SubproofMut` can't exist to mutate
//   the arena that would conflict with field access.
// - `SubproofMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for SubproofView<'_> {}
unsafe impl Send for SubproofView<'_> {}

impl<'a> ::__pb::ViewProxy<'a> for SubproofView<'a> {
  type Proxied = Subproof;

  fn as_view(&self) -> ::__pb::View<'a, Subproof> {
    *self
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Subproof> where 'a: 'shorter {
    self
  }
}

impl<'a> ::__pb::SettableValue<Subproof> for SubproofView<'a> {
  fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<Subproof>) {
    todo!()
  }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct SubproofMut<'a> {
  inner: ::__pb::__runtime::MutatorMessageRef<'a>,
}

// SAFETY:
// - `SubproofMut` does not perform any shared mutation.
// - `SubproofMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for SubproofMut<'_> {}

impl<'a> ::__pb::MutProxy<'a> for SubproofMut<'a> {
  fn as_mut(&mut self) -> ::__pb::Mut<'_, Subproof> {
    SubproofMut { inner: self.inner }
  }
  fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, Subproof> where 'a : 'shorter { self }
}

impl<'a> ::__pb::ViewProxy<'a> for SubproofMut<'a> {
  type Proxied = Subproof;
  fn as_view(&self) -> ::__pb::View<'_, Subproof> {
    SubproofView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Subproof> where 'a: 'shorter {
    SubproofView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
}

impl Subproof {
  pub fn new() -> Self {
    let arena = ::__pb::__runtime::Arena::new();
    Self {
      inner: ::__pb::__runtime::MessageInner {
        msg: unsafe { pilout_Subproof_new(arena.raw()) },
        arena,
      }
    }
  }

  pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
    let arena = ::__pb::__runtime::Arena::new();
    let mut len = 0;
    unsafe {
      let data = pilout_Subproof_serialize(self.inner.msg, arena.raw(), &mut len);
      ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
    }
  }
  pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
    let arena = ::__pb::__runtime::Arena::new();
    let msg = unsafe {
      pilout_Subproof_parse(data.as_ptr(), data.len(), arena.raw())
    };

    match msg {
      None => Err(::__pb::ParseError),
      Some(msg) => {
        // This assignment causes self.arena to be dropped and to deallocate
        // any previous message pointed/owned to by self.inner.msg.
        self.inner.arena = arena;
        self.inner.msg = msg;
        Ok(())
      }
    }
  }

  // name: optional string
  pub fn r#name(&self) -> &::__pb::ProtoStr {
    let view = unsafe { pilout_Subproof_name(self.inner.msg).as_ref() };
    // SAFETY: The runtime doesn't require ProtoStr to be UTF-8.
    unsafe { ::__pb::ProtoStr::from_utf8_unchecked(view) }
  }

  pub fn name_opt(&self) -> ::__pb::Optional<&::__pb::ProtoStr> {
    unsafe {
      let view = pilout_Subproof_name(self.inner.msg).as_ref();
      ::__pb::Optional::new(
        // SAFETY: The runtime doesn't require ProtoStr to be UTF-8.
        unsafe { ::__pb::ProtoStr::from_utf8_unchecked(view) }
         ,
        pilout_Subproof_has_name(self.inner.msg)
      )
    }
  }
  pub fn name_mut(&mut self) -> ::__pb::FieldEntry<'_, ::__pb::ProtoStr> {
    static VTABLE: ::__pb::__internal::BytesOptionalMutVTable = unsafe {
      ::__pb::__internal::BytesOptionalMutVTable::new(
        ::__pb::__internal::Private,
        pilout_Subproof_name,
        pilout_Subproof_set_name,
        pilout_Subproof_clear_name,
        b"",
      )
    };
    let out = unsafe {
      let has = pilout_Subproof_has_name(self.inner.msg);
      ::__pb::__internal::new_vtable_field_entry(
        ::__pb::__internal::Private,
        ::__pb::__runtime::MutatorMessageRef::new(
          ::__pb::__internal::Private, &mut self.inner),
        &VTABLE,
        has,
      )
    };
    ::__pb::ProtoStrMut::field_entry_from_bytes(
      ::__pb::__internal::Private, out
    )
  }

  // aggregable: optional bool
  pub fn r#aggregable(&self) -> bool {
    unsafe { pilout_Subproof_aggregable(self.inner.msg) }
  }
  pub fn r#aggregable_mut(&mut self) -> ::__pb::PrimitiveMut<'_, bool> {
    static VTABLE: ::__pb::__internal::PrimitiveVTable<bool> =
      ::__pb::__internal::PrimitiveVTable::new(
        ::__pb::__internal::Private,
        pilout_Subproof_aggregable,
        pilout_Subproof_set_aggregable,
      );

      ::__pb::PrimitiveMut::from_inner(
        ::__pb::__internal::Private,
        unsafe {
          ::__pb::__internal::RawVTableMutator::new(
            ::__pb::__internal::Private,
            ::__pb::__runtime::MutatorMessageRef::new(
              ::__pb::__internal::Private, &mut self.inner
            ),
            &VTABLE,
          )
        },
      )
  }

  // subproofvalues: repeated message pilout.SubproofValue
  // Unsupported! :(


  // airs: repeated message pilout.BasicAir
  // Unsupported! :(



}  // impl Subproof

impl ::__std::ops::Drop for Subproof {
  fn drop(&mut self) {
  }
}

extern "C" {
  fn pilout_Subproof_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
  fn pilout_Subproof_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
  fn pilout_Subproof_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_Subproof_has_name(raw_msg: ::__pb::__internal::RawMessage) -> bool;
  fn pilout_Subproof_name(raw_msg: ::__pb::__internal::RawMessage) -> ::__pb::__internal::PtrAndLen;
  fn pilout_Subproof_set_name(raw_msg: ::__pb::__internal::RawMessage, val: ::__pb::__internal::PtrAndLen);
  fn pilout_Subproof_clear_name(raw_msg: ::__pb::__internal::RawMessage);

  fn pilout_Subproof_aggregable(raw_msg: ::__pb::__internal::RawMessage) -> bool;
  fn pilout_Subproof_set_aggregable(raw_msg: ::__pb::__internal::RawMessage, val: bool);
  fn pilout_Subproof_clear_aggregable(raw_msg: ::__pb::__internal::RawMessage);




}  // extern "C" for Subproof


#[allow(non_camel_case_types)]
// TODO: Implement support for debug redaction
#[derive(Debug)]
pub struct SubproofValue {
  inner: ::__pb::__runtime::MessageInner
}

// SAFETY:
// - `SubproofValue` does not provide shared mutation with its arena.
// - `SubproofValueMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena that would conflict with
//   field access is impossible.
unsafe impl Sync for SubproofValue {}

impl ::__pb::Proxied for SubproofValue {
  type View<'a> = SubproofValueView<'a>;
  type Mut<'a> = SubproofValueMut<'a>;
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub struct SubproofValueView<'a> {
  msg: ::__pb::__internal::RawMessage,
  _phantom: ::__std::marker::PhantomData<&'a ()>,
}

impl<'a> SubproofValueView<'a> {
  #[doc(hidden)]
  pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
    Self { msg, _phantom: std::marker::PhantomData }
  }
}

// SAFETY:
// - `SubproofValueView` does not perform any mutation.
// - While a `SubproofValueView` exists, a `SubproofValueMut` can't exist to mutate
//   the arena that would conflict with field access.
// - `SubproofValueMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for SubproofValueView<'_> {}
unsafe impl Send for SubproofValueView<'_> {}

impl<'a> ::__pb::ViewProxy<'a> for SubproofValueView<'a> {
  type Proxied = SubproofValue;

  fn as_view(&self) -> ::__pb::View<'a, SubproofValue> {
    *self
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, SubproofValue> where 'a: 'shorter {
    self
  }
}

impl<'a> ::__pb::SettableValue<SubproofValue> for SubproofValueView<'a> {
  fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<SubproofValue>) {
    todo!()
  }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct SubproofValueMut<'a> {
  inner: ::__pb::__runtime::MutatorMessageRef<'a>,
}

// SAFETY:
// - `SubproofValueMut` does not perform any shared mutation.
// - `SubproofValueMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for SubproofValueMut<'_> {}

impl<'a> ::__pb::MutProxy<'a> for SubproofValueMut<'a> {
  fn as_mut(&mut self) -> ::__pb::Mut<'_, SubproofValue> {
    SubproofValueMut { inner: self.inner }
  }
  fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, SubproofValue> where 'a : 'shorter { self }
}

impl<'a> ::__pb::ViewProxy<'a> for SubproofValueMut<'a> {
  type Proxied = SubproofValue;
  fn as_view(&self) -> ::__pb::View<'_, SubproofValue> {
    SubproofValueView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, SubproofValue> where 'a: 'shorter {
    SubproofValueView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
}

impl SubproofValue {
  pub fn new() -> Self {
    let arena = ::__pb::__runtime::Arena::new();
    Self {
      inner: ::__pb::__runtime::MessageInner {
        msg: unsafe { pilout_SubproofValue_new(arena.raw()) },
        arena,
      }
    }
  }

  pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
    let arena = ::__pb::__runtime::Arena::new();
    let mut len = 0;
    unsafe {
      let data = pilout_SubproofValue_serialize(self.inner.msg, arena.raw(), &mut len);
      ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
    }
  }
  pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
    let arena = ::__pb::__runtime::Arena::new();
    let msg = unsafe {
      pilout_SubproofValue_parse(data.as_ptr(), data.len(), arena.raw())
    };

    match msg {
      None => Err(::__pb::ParseError),
      Some(msg) => {
        // This assignment causes self.arena to be dropped and to deallocate
        // any previous message pointed/owned to by self.inner.msg.
        self.inner.arena = arena;
        self.inner.msg = msg;
        Ok(())
      }
    }
  }

  // aggType: optional enum pilout.AggregationType
  // Unsupported! :(



}  // impl SubproofValue

impl ::__std::ops::Drop for SubproofValue {
  fn drop(&mut self) {
  }
}

extern "C" {
  fn pilout_SubproofValue_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
  fn pilout_SubproofValue_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
  fn pilout_SubproofValue_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;



}  // extern "C" for SubproofValue


#[allow(non_camel_case_types)]
// TODO: Implement support for debug redaction
#[derive(Debug)]
pub struct PublicTable {
  inner: ::__pb::__runtime::MessageInner
}

// SAFETY:
// - `PublicTable` does not provide shared mutation with its arena.
// - `PublicTableMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena that would conflict with
//   field access is impossible.
unsafe impl Sync for PublicTable {}

impl ::__pb::Proxied for PublicTable {
  type View<'a> = PublicTableView<'a>;
  type Mut<'a> = PublicTableMut<'a>;
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub struct PublicTableView<'a> {
  msg: ::__pb::__internal::RawMessage,
  _phantom: ::__std::marker::PhantomData<&'a ()>,
}

impl<'a> PublicTableView<'a> {
  #[doc(hidden)]
  pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
    Self { msg, _phantom: std::marker::PhantomData }
  }
  pub fn r#numCols(&self) -> u32 { unsafe {
    pilout_PublicTable_numCols(self.msg)
  } }

  pub fn r#maxRows(&self) -> u32 { unsafe {
    pilout_PublicTable_maxRows(self.msg)
  } }

}

// SAFETY:
// - `PublicTableView` does not perform any mutation.
// - While a `PublicTableView` exists, a `PublicTableMut` can't exist to mutate
//   the arena that would conflict with field access.
// - `PublicTableMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for PublicTableView<'_> {}
unsafe impl Send for PublicTableView<'_> {}

impl<'a> ::__pb::ViewProxy<'a> for PublicTableView<'a> {
  type Proxied = PublicTable;

  fn as_view(&self) -> ::__pb::View<'a, PublicTable> {
    *self
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, PublicTable> where 'a: 'shorter {
    self
  }
}

impl<'a> ::__pb::SettableValue<PublicTable> for PublicTableView<'a> {
  fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<PublicTable>) {
    todo!()
  }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct PublicTableMut<'a> {
  inner: ::__pb::__runtime::MutatorMessageRef<'a>,
}

// SAFETY:
// - `PublicTableMut` does not perform any shared mutation.
// - `PublicTableMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for PublicTableMut<'_> {}

impl<'a> ::__pb::MutProxy<'a> for PublicTableMut<'a> {
  fn as_mut(&mut self) -> ::__pb::Mut<'_, PublicTable> {
    PublicTableMut { inner: self.inner }
  }
  fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, PublicTable> where 'a : 'shorter { self }
}

impl<'a> ::__pb::ViewProxy<'a> for PublicTableMut<'a> {
  type Proxied = PublicTable;
  fn as_view(&self) -> ::__pb::View<'_, PublicTable> {
    PublicTableView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, PublicTable> where 'a: 'shorter {
    PublicTableView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
}

impl PublicTable {
  pub fn new() -> Self {
    let arena = ::__pb::__runtime::Arena::new();
    Self {
      inner: ::__pb::__runtime::MessageInner {
        msg: unsafe { pilout_PublicTable_new(arena.raw()) },
        arena,
      }
    }
  }

  pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
    let arena = ::__pb::__runtime::Arena::new();
    let mut len = 0;
    unsafe {
      let data = pilout_PublicTable_serialize(self.inner.msg, arena.raw(), &mut len);
      ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
    }
  }
  pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
    let arena = ::__pb::__runtime::Arena::new();
    let msg = unsafe {
      pilout_PublicTable_parse(data.as_ptr(), data.len(), arena.raw())
    };

    match msg {
      None => Err(::__pb::ParseError),
      Some(msg) => {
        // This assignment causes self.arena to be dropped and to deallocate
        // any previous message pointed/owned to by self.inner.msg.
        self.inner.arena = arena;
        self.inner.msg = msg;
        Ok(())
      }
    }
  }

  // numCols: optional uint32
  pub fn r#numCols(&self) -> u32 {
    unsafe { pilout_PublicTable_numCols(self.inner.msg) }
  }
  pub fn r#numCols_mut(&mut self) -> ::__pb::PrimitiveMut<'_, u32> {
    static VTABLE: ::__pb::__internal::PrimitiveVTable<u32> =
      ::__pb::__internal::PrimitiveVTable::new(
        ::__pb::__internal::Private,
        pilout_PublicTable_numCols,
        pilout_PublicTable_set_numCols,
      );

      ::__pb::PrimitiveMut::from_inner(
        ::__pb::__internal::Private,
        unsafe {
          ::__pb::__internal::RawVTableMutator::new(
            ::__pb::__internal::Private,
            ::__pb::__runtime::MutatorMessageRef::new(
              ::__pb::__internal::Private, &mut self.inner
            ),
            &VTABLE,
          )
        },
      )
  }

  // maxRows: optional uint32
  pub fn r#maxRows(&self) -> u32 {
    unsafe { pilout_PublicTable_maxRows(self.inner.msg) }
  }
  pub fn r#maxRows_mut(&mut self) -> ::__pb::PrimitiveMut<'_, u32> {
    static VTABLE: ::__pb::__internal::PrimitiveVTable<u32> =
      ::__pb::__internal::PrimitiveVTable::new(
        ::__pb::__internal::Private,
        pilout_PublicTable_maxRows,
        pilout_PublicTable_set_maxRows,
      );

      ::__pb::PrimitiveMut::from_inner(
        ::__pb::__internal::Private,
        unsafe {
          ::__pb::__internal::RawVTableMutator::new(
            ::__pb::__internal::Private,
            ::__pb::__runtime::MutatorMessageRef::new(
              ::__pb::__internal::Private, &mut self.inner
            ),
            &VTABLE,
          )
        },
      )
  }

  // aggType: optional enum pilout.AggregationType
  // Unsupported! :(


  // rowExpressionIdx: optional message pilout.GlobalOperand.Expression
  pub fn r#rowExpressionIdx(&self) -> crate::pilout::GlobalOperand_::ExpressionView {
    let submsg = unsafe { pilout_PublicTable_rowExpressionIdx(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::GlobalOperand_::ExpressionView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::GlobalOperand_::ExpressionView::new(::__pb::__internal::Private, field),
      }
  }


}  // impl PublicTable

impl ::__std::ops::Drop for PublicTable {
  fn drop(&mut self) {
  }
}

extern "C" {
  fn pilout_PublicTable_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
  fn pilout_PublicTable_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
  fn pilout_PublicTable_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_PublicTable_numCols(raw_msg: ::__pb::__internal::RawMessage) -> u32;
  fn pilout_PublicTable_set_numCols(raw_msg: ::__pb::__internal::RawMessage, val: u32);
  fn pilout_PublicTable_clear_numCols(raw_msg: ::__pb::__internal::RawMessage);

  fn pilout_PublicTable_maxRows(raw_msg: ::__pb::__internal::RawMessage) -> u32;
  fn pilout_PublicTable_set_maxRows(raw_msg: ::__pb::__internal::RawMessage, val: u32);
  fn pilout_PublicTable_clear_maxRows(raw_msg: ::__pb::__internal::RawMessage);


  fn pilout_PublicTable_rowExpressionIdx(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;


}  // extern "C" for PublicTable


#[allow(non_camel_case_types)]
// TODO: Implement support for debug redaction
#[derive(Debug)]
pub struct GlobalConstraint {
  inner: ::__pb::__runtime::MessageInner
}

// SAFETY:
// - `GlobalConstraint` does not provide shared mutation with its arena.
// - `GlobalConstraintMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena that would conflict with
//   field access is impossible.
unsafe impl Sync for GlobalConstraint {}

impl ::__pb::Proxied for GlobalConstraint {
  type View<'a> = GlobalConstraintView<'a>;
  type Mut<'a> = GlobalConstraintMut<'a>;
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub struct GlobalConstraintView<'a> {
  msg: ::__pb::__internal::RawMessage,
  _phantom: ::__std::marker::PhantomData<&'a ()>,
}

impl<'a> GlobalConstraintView<'a> {
  #[doc(hidden)]
  pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
    Self { msg, _phantom: std::marker::PhantomData }
  }
}

// SAFETY:
// - `GlobalConstraintView` does not perform any mutation.
// - While a `GlobalConstraintView` exists, a `GlobalConstraintMut` can't exist to mutate
//   the arena that would conflict with field access.
// - `GlobalConstraintMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for GlobalConstraintView<'_> {}
unsafe impl Send for GlobalConstraintView<'_> {}

impl<'a> ::__pb::ViewProxy<'a> for GlobalConstraintView<'a> {
  type Proxied = GlobalConstraint;

  fn as_view(&self) -> ::__pb::View<'a, GlobalConstraint> {
    *self
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, GlobalConstraint> where 'a: 'shorter {
    self
  }
}

impl<'a> ::__pb::SettableValue<GlobalConstraint> for GlobalConstraintView<'a> {
  fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<GlobalConstraint>) {
    todo!()
  }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct GlobalConstraintMut<'a> {
  inner: ::__pb::__runtime::MutatorMessageRef<'a>,
}

// SAFETY:
// - `GlobalConstraintMut` does not perform any shared mutation.
// - `GlobalConstraintMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for GlobalConstraintMut<'_> {}

impl<'a> ::__pb::MutProxy<'a> for GlobalConstraintMut<'a> {
  fn as_mut(&mut self) -> ::__pb::Mut<'_, GlobalConstraint> {
    GlobalConstraintMut { inner: self.inner }
  }
  fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, GlobalConstraint> where 'a : 'shorter { self }
}

impl<'a> ::__pb::ViewProxy<'a> for GlobalConstraintMut<'a> {
  type Proxied = GlobalConstraint;
  fn as_view(&self) -> ::__pb::View<'_, GlobalConstraint> {
    GlobalConstraintView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, GlobalConstraint> where 'a: 'shorter {
    GlobalConstraintView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
}

impl GlobalConstraint {
  pub fn new() -> Self {
    let arena = ::__pb::__runtime::Arena::new();
    Self {
      inner: ::__pb::__runtime::MessageInner {
        msg: unsafe { pilout_GlobalConstraint_new(arena.raw()) },
        arena,
      }
    }
  }

  pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
    let arena = ::__pb::__runtime::Arena::new();
    let mut len = 0;
    unsafe {
      let data = pilout_GlobalConstraint_serialize(self.inner.msg, arena.raw(), &mut len);
      ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
    }
  }
  pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
    let arena = ::__pb::__runtime::Arena::new();
    let msg = unsafe {
      pilout_GlobalConstraint_parse(data.as_ptr(), data.len(), arena.raw())
    };

    match msg {
      None => Err(::__pb::ParseError),
      Some(msg) => {
        // This assignment causes self.arena to be dropped and to deallocate
        // any previous message pointed/owned to by self.inner.msg.
        self.inner.arena = arena;
        self.inner.msg = msg;
        Ok(())
      }
    }
  }

  // expressionIdx: optional message pilout.GlobalOperand.Expression
  pub fn r#expressionIdx(&self) -> crate::pilout::GlobalOperand_::ExpressionView {
    let submsg = unsafe { pilout_GlobalConstraint_expressionIdx(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::GlobalOperand_::ExpressionView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::GlobalOperand_::ExpressionView::new(::__pb::__internal::Private, field),
      }
  }

  // debugLine: optional string
  pub fn r#debugLine(&self) -> &::__pb::ProtoStr {
    let view = unsafe { pilout_GlobalConstraint_debugLine(self.inner.msg).as_ref() };
    // SAFETY: The runtime doesn't require ProtoStr to be UTF-8.
    unsafe { ::__pb::ProtoStr::from_utf8_unchecked(view) }
  }

  pub fn debugLine_opt(&self) -> ::__pb::Optional<&::__pb::ProtoStr> {
    unsafe {
      let view = pilout_GlobalConstraint_debugLine(self.inner.msg).as_ref();
      ::__pb::Optional::new(
        // SAFETY: The runtime doesn't require ProtoStr to be UTF-8.
        unsafe { ::__pb::ProtoStr::from_utf8_unchecked(view) }
         ,
        pilout_GlobalConstraint_has_debugLine(self.inner.msg)
      )
    }
  }
  pub fn debugLine_mut(&mut self) -> ::__pb::FieldEntry<'_, ::__pb::ProtoStr> {
    static VTABLE: ::__pb::__internal::BytesOptionalMutVTable = unsafe {
      ::__pb::__internal::BytesOptionalMutVTable::new(
        ::__pb::__internal::Private,
        pilout_GlobalConstraint_debugLine,
        pilout_GlobalConstraint_set_debugLine,
        pilout_GlobalConstraint_clear_debugLine,
        b"",
      )
    };
    let out = unsafe {
      let has = pilout_GlobalConstraint_has_debugLine(self.inner.msg);
      ::__pb::__internal::new_vtable_field_entry(
        ::__pb::__internal::Private,
        ::__pb::__runtime::MutatorMessageRef::new(
          ::__pb::__internal::Private, &mut self.inner),
        &VTABLE,
        has,
      )
    };
    ::__pb::ProtoStrMut::field_entry_from_bytes(
      ::__pb::__internal::Private, out
    )
  }


}  // impl GlobalConstraint

impl ::__std::ops::Drop for GlobalConstraint {
  fn drop(&mut self) {
  }
}

extern "C" {
  fn pilout_GlobalConstraint_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
  fn pilout_GlobalConstraint_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
  fn pilout_GlobalConstraint_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_GlobalConstraint_expressionIdx(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_GlobalConstraint_has_debugLine(raw_msg: ::__pb::__internal::RawMessage) -> bool;
  fn pilout_GlobalConstraint_debugLine(raw_msg: ::__pb::__internal::RawMessage) -> ::__pb::__internal::PtrAndLen;
  fn pilout_GlobalConstraint_set_debugLine(raw_msg: ::__pb::__internal::RawMessage, val: ::__pb::__internal::PtrAndLen);
  fn pilout_GlobalConstraint_clear_debugLine(raw_msg: ::__pb::__internal::RawMessage);


}  // extern "C" for GlobalConstraint


#[allow(non_camel_case_types)]
// TODO: Implement support for debug redaction
#[derive(Debug)]
pub struct GlobalExpression {
  inner: ::__pb::__runtime::MessageInner
}

// SAFETY:
// - `GlobalExpression` does not provide shared mutation with its arena.
// - `GlobalExpressionMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena that would conflict with
//   field access is impossible.
unsafe impl Sync for GlobalExpression {}

impl ::__pb::Proxied for GlobalExpression {
  type View<'a> = GlobalExpressionView<'a>;
  type Mut<'a> = GlobalExpressionMut<'a>;
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub struct GlobalExpressionView<'a> {
  msg: ::__pb::__internal::RawMessage,
  _phantom: ::__std::marker::PhantomData<&'a ()>,
}

impl<'a> GlobalExpressionView<'a> {
  #[doc(hidden)]
  pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
    Self { msg, _phantom: std::marker::PhantomData }
  }
}

// SAFETY:
// - `GlobalExpressionView` does not perform any mutation.
// - While a `GlobalExpressionView` exists, a `GlobalExpressionMut` can't exist to mutate
//   the arena that would conflict with field access.
// - `GlobalExpressionMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for GlobalExpressionView<'_> {}
unsafe impl Send for GlobalExpressionView<'_> {}

impl<'a> ::__pb::ViewProxy<'a> for GlobalExpressionView<'a> {
  type Proxied = GlobalExpression;

  fn as_view(&self) -> ::__pb::View<'a, GlobalExpression> {
    *self
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, GlobalExpression> where 'a: 'shorter {
    self
  }
}

impl<'a> ::__pb::SettableValue<GlobalExpression> for GlobalExpressionView<'a> {
  fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<GlobalExpression>) {
    todo!()
  }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct GlobalExpressionMut<'a> {
  inner: ::__pb::__runtime::MutatorMessageRef<'a>,
}

// SAFETY:
// - `GlobalExpressionMut` does not perform any shared mutation.
// - `GlobalExpressionMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for GlobalExpressionMut<'_> {}

impl<'a> ::__pb::MutProxy<'a> for GlobalExpressionMut<'a> {
  fn as_mut(&mut self) -> ::__pb::Mut<'_, GlobalExpression> {
    GlobalExpressionMut { inner: self.inner }
  }
  fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, GlobalExpression> where 'a : 'shorter { self }
}

impl<'a> ::__pb::ViewProxy<'a> for GlobalExpressionMut<'a> {
  type Proxied = GlobalExpression;
  fn as_view(&self) -> ::__pb::View<'_, GlobalExpression> {
    GlobalExpressionView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, GlobalExpression> where 'a: 'shorter {
    GlobalExpressionView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
}

impl GlobalExpression {
  pub fn new() -> Self {
    let arena = ::__pb::__runtime::Arena::new();
    Self {
      inner: ::__pb::__runtime::MessageInner {
        msg: unsafe { pilout_GlobalExpression_new(arena.raw()) },
        arena,
      }
    }
  }

  pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
    let arena = ::__pb::__runtime::Arena::new();
    let mut len = 0;
    unsafe {
      let data = pilout_GlobalExpression_serialize(self.inner.msg, arena.raw(), &mut len);
      ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
    }
  }
  pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
    let arena = ::__pb::__runtime::Arena::new();
    let msg = unsafe {
      pilout_GlobalExpression_parse(data.as_ptr(), data.len(), arena.raw())
    };

    match msg {
      None => Err(::__pb::ParseError),
      Some(msg) => {
        // This assignment causes self.arena to be dropped and to deallocate
        // any previous message pointed/owned to by self.inner.msg.
        self.inner.arena = arena;
        self.inner.msg = msg;
        Ok(())
      }
    }
  }

  // add: optional message pilout.GlobalExpression.Add
  pub fn r#add(&self) -> crate::pilout::GlobalExpression_::AddView {
    let submsg = unsafe { pilout_GlobalExpression_add(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::GlobalExpression_::AddView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::GlobalExpression_::AddView::new(::__pb::__internal::Private, field),
      }
  }

  // sub: optional message pilout.GlobalExpression.Sub
  pub fn r#sub(&self) -> crate::pilout::GlobalExpression_::SubView {
    let submsg = unsafe { pilout_GlobalExpression_sub(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::GlobalExpression_::SubView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::GlobalExpression_::SubView::new(::__pb::__internal::Private, field),
      }
  }

  // mul: optional message pilout.GlobalExpression.Mul
  pub fn r#mul(&self) -> crate::pilout::GlobalExpression_::MulView {
    let submsg = unsafe { pilout_GlobalExpression_mul(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::GlobalExpression_::MulView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::GlobalExpression_::MulView::new(::__pb::__internal::Private, field),
      }
  }

  // neg: optional message pilout.GlobalExpression.Neg
  pub fn r#neg(&self) -> crate::pilout::GlobalExpression_::NegView {
    let submsg = unsafe { pilout_GlobalExpression_neg(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::GlobalExpression_::NegView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::GlobalExpression_::NegView::new(::__pb::__internal::Private, field),
      }
  }


  pub fn r#operation(&self) -> GlobalExpression_::Operation {
    match unsafe { pilout_GlobalExpression_operation_case(self.inner.msg) } {
      _ => GlobalExpression_::Operation::not_set(std::marker::PhantomData)
    }
  }

  pub fn r#operation_mut(&mut self) -> GlobalExpression_::OperationMut {
    match unsafe { pilout_GlobalExpression_operation_case(self.inner.msg) } {
      _ => GlobalExpression_::OperationMut::not_set(std::marker::PhantomData)
    }
  }

}  // impl GlobalExpression

impl ::__std::ops::Drop for GlobalExpression {
  fn drop(&mut self) {
  }
}

extern "C" {
  fn pilout_GlobalExpression_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
  fn pilout_GlobalExpression_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
  fn pilout_GlobalExpression_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_GlobalExpression_add(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_GlobalExpression_sub(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_GlobalExpression_mul(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_GlobalExpression_neg(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;


  fn pilout_GlobalExpression_operation_case(raw_msg: ::__pb::__internal::RawMessage) -> GlobalExpression_::OperationCase;

}  // extern "C" for GlobalExpression

#[allow(non_snake_case)]
pub mod GlobalExpression_ {
  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct Add {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `Add` does not provide shared mutation with its arena.
  // - `AddMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for Add {}

  impl ::__pb::Proxied for Add {
    type View<'a> = AddView<'a>;
    type Mut<'a> = AddMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct AddView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> AddView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
  }

  // SAFETY:
  // - `AddView` does not perform any mutation.
  // - While a `AddView` exists, a `AddMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `AddMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for AddView<'_> {}
  unsafe impl Send for AddView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for AddView<'a> {
    type Proxied = Add;

    fn as_view(&self) -> ::__pb::View<'a, Add> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Add> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<Add> for AddView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<Add>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct AddMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `AddMut` does not perform any shared mutation.
  // - `AddMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for AddMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for AddMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, Add> {
      AddMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, Add> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for AddMut<'a> {
    type Proxied = Add;
    fn as_view(&self) -> ::__pb::View<'_, Add> {
      AddView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Add> where 'a: 'shorter {
      AddView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl Add {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_GlobalExpression_Add_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_GlobalExpression_Add_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_GlobalExpression_Add_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // lhs: optional message pilout.GlobalOperand
    pub fn r#lhs(&self) -> crate::pilout::GlobalOperandView {
      let submsg = unsafe { pilout_GlobalExpression_Add_lhs(self.inner.msg) };
      // For upb, getters return null if the field is unset, so we need to
      // check for null and return the default instance manually. Note that
      // a null ptr received from upb manifests as Option::None
      match submsg {
          // TODO:(b/304357029)
          None => crate::pilout::GlobalOperandView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
          Some(field) => crate::pilout::GlobalOperandView::new(::__pb::__internal::Private, field),
        }
    }

    // rhs: optional message pilout.GlobalOperand
    pub fn r#rhs(&self) -> crate::pilout::GlobalOperandView {
      let submsg = unsafe { pilout_GlobalExpression_Add_rhs(self.inner.msg) };
      // For upb, getters return null if the field is unset, so we need to
      // check for null and return the default instance manually. Note that
      // a null ptr received from upb manifests as Option::None
      match submsg {
          // TODO:(b/304357029)
          None => crate::pilout::GlobalOperandView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
          Some(field) => crate::pilout::GlobalOperandView::new(::__pb::__internal::Private, field),
        }
    }


  }  // impl Add

  impl ::__std::ops::Drop for Add {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_GlobalExpression_Add_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_GlobalExpression_Add_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_GlobalExpression_Add_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_GlobalExpression_Add_lhs(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_GlobalExpression_Add_rhs(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;


  }  // extern "C" for Add

  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct Sub {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `Sub` does not provide shared mutation with its arena.
  // - `SubMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for Sub {}

  impl ::__pb::Proxied for Sub {
    type View<'a> = SubView<'a>;
    type Mut<'a> = SubMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct SubView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> SubView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
  }

  // SAFETY:
  // - `SubView` does not perform any mutation.
  // - While a `SubView` exists, a `SubMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `SubMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for SubView<'_> {}
  unsafe impl Send for SubView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for SubView<'a> {
    type Proxied = Sub;

    fn as_view(&self) -> ::__pb::View<'a, Sub> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Sub> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<Sub> for SubView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<Sub>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct SubMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `SubMut` does not perform any shared mutation.
  // - `SubMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for SubMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for SubMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, Sub> {
      SubMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, Sub> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for SubMut<'a> {
    type Proxied = Sub;
    fn as_view(&self) -> ::__pb::View<'_, Sub> {
      SubView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Sub> where 'a: 'shorter {
      SubView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl Sub {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_GlobalExpression_Sub_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_GlobalExpression_Sub_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_GlobalExpression_Sub_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // lhs: optional message pilout.GlobalOperand
    pub fn r#lhs(&self) -> crate::pilout::GlobalOperandView {
      let submsg = unsafe { pilout_GlobalExpression_Sub_lhs(self.inner.msg) };
      // For upb, getters return null if the field is unset, so we need to
      // check for null and return the default instance manually. Note that
      // a null ptr received from upb manifests as Option::None
      match submsg {
          // TODO:(b/304357029)
          None => crate::pilout::GlobalOperandView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
          Some(field) => crate::pilout::GlobalOperandView::new(::__pb::__internal::Private, field),
        }
    }

    // rhs: optional message pilout.GlobalOperand
    pub fn r#rhs(&self) -> crate::pilout::GlobalOperandView {
      let submsg = unsafe { pilout_GlobalExpression_Sub_rhs(self.inner.msg) };
      // For upb, getters return null if the field is unset, so we need to
      // check for null and return the default instance manually. Note that
      // a null ptr received from upb manifests as Option::None
      match submsg {
          // TODO:(b/304357029)
          None => crate::pilout::GlobalOperandView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
          Some(field) => crate::pilout::GlobalOperandView::new(::__pb::__internal::Private, field),
        }
    }


  }  // impl Sub

  impl ::__std::ops::Drop for Sub {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_GlobalExpression_Sub_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_GlobalExpression_Sub_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_GlobalExpression_Sub_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_GlobalExpression_Sub_lhs(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_GlobalExpression_Sub_rhs(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;


  }  // extern "C" for Sub

  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct Mul {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `Mul` does not provide shared mutation with its arena.
  // - `MulMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for Mul {}

  impl ::__pb::Proxied for Mul {
    type View<'a> = MulView<'a>;
    type Mut<'a> = MulMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct MulView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> MulView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
  }

  // SAFETY:
  // - `MulView` does not perform any mutation.
  // - While a `MulView` exists, a `MulMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `MulMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for MulView<'_> {}
  unsafe impl Send for MulView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for MulView<'a> {
    type Proxied = Mul;

    fn as_view(&self) -> ::__pb::View<'a, Mul> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Mul> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<Mul> for MulView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<Mul>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct MulMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `MulMut` does not perform any shared mutation.
  // - `MulMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for MulMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for MulMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, Mul> {
      MulMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, Mul> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for MulMut<'a> {
    type Proxied = Mul;
    fn as_view(&self) -> ::__pb::View<'_, Mul> {
      MulView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Mul> where 'a: 'shorter {
      MulView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl Mul {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_GlobalExpression_Mul_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_GlobalExpression_Mul_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_GlobalExpression_Mul_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // lhs: optional message pilout.GlobalOperand
    pub fn r#lhs(&self) -> crate::pilout::GlobalOperandView {
      let submsg = unsafe { pilout_GlobalExpression_Mul_lhs(self.inner.msg) };
      // For upb, getters return null if the field is unset, so we need to
      // check for null and return the default instance manually. Note that
      // a null ptr received from upb manifests as Option::None
      match submsg {
          // TODO:(b/304357029)
          None => crate::pilout::GlobalOperandView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
          Some(field) => crate::pilout::GlobalOperandView::new(::__pb::__internal::Private, field),
        }
    }

    // rhs: optional message pilout.GlobalOperand
    pub fn r#rhs(&self) -> crate::pilout::GlobalOperandView {
      let submsg = unsafe { pilout_GlobalExpression_Mul_rhs(self.inner.msg) };
      // For upb, getters return null if the field is unset, so we need to
      // check for null and return the default instance manually. Note that
      // a null ptr received from upb manifests as Option::None
      match submsg {
          // TODO:(b/304357029)
          None => crate::pilout::GlobalOperandView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
          Some(field) => crate::pilout::GlobalOperandView::new(::__pb::__internal::Private, field),
        }
    }


  }  // impl Mul

  impl ::__std::ops::Drop for Mul {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_GlobalExpression_Mul_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_GlobalExpression_Mul_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_GlobalExpression_Mul_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_GlobalExpression_Mul_lhs(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_GlobalExpression_Mul_rhs(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;


  }  // extern "C" for Mul

  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct Neg {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `Neg` does not provide shared mutation with its arena.
  // - `NegMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for Neg {}

  impl ::__pb::Proxied for Neg {
    type View<'a> = NegView<'a>;
    type Mut<'a> = NegMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct NegView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> NegView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
  }

  // SAFETY:
  // - `NegView` does not perform any mutation.
  // - While a `NegView` exists, a `NegMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `NegMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for NegView<'_> {}
  unsafe impl Send for NegView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for NegView<'a> {
    type Proxied = Neg;

    fn as_view(&self) -> ::__pb::View<'a, Neg> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Neg> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<Neg> for NegView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<Neg>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct NegMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `NegMut` does not perform any shared mutation.
  // - `NegMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for NegMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for NegMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, Neg> {
      NegMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, Neg> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for NegMut<'a> {
    type Proxied = Neg;
    fn as_view(&self) -> ::__pb::View<'_, Neg> {
      NegView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Neg> where 'a: 'shorter {
      NegView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl Neg {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_GlobalExpression_Neg_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_GlobalExpression_Neg_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_GlobalExpression_Neg_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // value: optional message pilout.GlobalOperand
    pub fn r#value(&self) -> crate::pilout::GlobalOperandView {
      let submsg = unsafe { pilout_GlobalExpression_Neg_value(self.inner.msg) };
      // For upb, getters return null if the field is unset, so we need to
      // check for null and return the default instance manually. Note that
      // a null ptr received from upb manifests as Option::None
      match submsg {
          // TODO:(b/304357029)
          None => crate::pilout::GlobalOperandView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
          Some(field) => crate::pilout::GlobalOperandView::new(::__pb::__internal::Private, field),
        }
    }


  }  // impl Neg

  impl ::__std::ops::Drop for Neg {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_GlobalExpression_Neg_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_GlobalExpression_Neg_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_GlobalExpression_Neg_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_GlobalExpression_Neg_value(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;


  }  // extern "C" for Neg


  #[non_exhaustive]
  #[derive(Debug)]
  #[allow(dead_code)]
  #[repr(isize)]
  pub enum Operation<'msg> {

    #[allow(non_camel_case_types)]
    not_set(std::marker::PhantomData<&'msg ()>) = 0
  }

  #[non_exhaustive]
  #[derive(Debug)]
  #[allow(dead_code)]
  #[repr(isize)]
  pub enum OperationMut<'msg> {

    #[allow(non_camel_case_types)]
    not_set(std::marker::PhantomData<&'msg ()>) = 0
  }
  #[repr(C)]
  #[derive(Debug, Copy, Clone, PartialEq, Eq)]
  pub(super) enum OperationCase {
    Add = 1,
    Sub = 2,
    Mul = 3,
    Neg = 4,

    #[allow(non_camel_case_types)]
    not_set = 0
  }
}  // mod GlobalExpression_

#[allow(non_camel_case_types)]
// TODO: Implement support for debug redaction
#[derive(Debug)]
pub struct GlobalOperand {
  inner: ::__pb::__runtime::MessageInner
}

// SAFETY:
// - `GlobalOperand` does not provide shared mutation with its arena.
// - `GlobalOperandMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena that would conflict with
//   field access is impossible.
unsafe impl Sync for GlobalOperand {}

impl ::__pb::Proxied for GlobalOperand {
  type View<'a> = GlobalOperandView<'a>;
  type Mut<'a> = GlobalOperandMut<'a>;
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub struct GlobalOperandView<'a> {
  msg: ::__pb::__internal::RawMessage,
  _phantom: ::__std::marker::PhantomData<&'a ()>,
}

impl<'a> GlobalOperandView<'a> {
  #[doc(hidden)]
  pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
    Self { msg, _phantom: std::marker::PhantomData }
  }
}

// SAFETY:
// - `GlobalOperandView` does not perform any mutation.
// - While a `GlobalOperandView` exists, a `GlobalOperandMut` can't exist to mutate
//   the arena that would conflict with field access.
// - `GlobalOperandMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for GlobalOperandView<'_> {}
unsafe impl Send for GlobalOperandView<'_> {}

impl<'a> ::__pb::ViewProxy<'a> for GlobalOperandView<'a> {
  type Proxied = GlobalOperand;

  fn as_view(&self) -> ::__pb::View<'a, GlobalOperand> {
    *self
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, GlobalOperand> where 'a: 'shorter {
    self
  }
}

impl<'a> ::__pb::SettableValue<GlobalOperand> for GlobalOperandView<'a> {
  fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<GlobalOperand>) {
    todo!()
  }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct GlobalOperandMut<'a> {
  inner: ::__pb::__runtime::MutatorMessageRef<'a>,
}

// SAFETY:
// - `GlobalOperandMut` does not perform any shared mutation.
// - `GlobalOperandMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for GlobalOperandMut<'_> {}

impl<'a> ::__pb::MutProxy<'a> for GlobalOperandMut<'a> {
  fn as_mut(&mut self) -> ::__pb::Mut<'_, GlobalOperand> {
    GlobalOperandMut { inner: self.inner }
  }
  fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, GlobalOperand> where 'a : 'shorter { self }
}

impl<'a> ::__pb::ViewProxy<'a> for GlobalOperandMut<'a> {
  type Proxied = GlobalOperand;
  fn as_view(&self) -> ::__pb::View<'_, GlobalOperand> {
    GlobalOperandView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, GlobalOperand> where 'a: 'shorter {
    GlobalOperandView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
}

impl GlobalOperand {
  pub fn new() -> Self {
    let arena = ::__pb::__runtime::Arena::new();
    Self {
      inner: ::__pb::__runtime::MessageInner {
        msg: unsafe { pilout_GlobalOperand_new(arena.raw()) },
        arena,
      }
    }
  }

  pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
    let arena = ::__pb::__runtime::Arena::new();
    let mut len = 0;
    unsafe {
      let data = pilout_GlobalOperand_serialize(self.inner.msg, arena.raw(), &mut len);
      ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
    }
  }
  pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
    let arena = ::__pb::__runtime::Arena::new();
    let msg = unsafe {
      pilout_GlobalOperand_parse(data.as_ptr(), data.len(), arena.raw())
    };

    match msg {
      None => Err(::__pb::ParseError),
      Some(msg) => {
        // This assignment causes self.arena to be dropped and to deallocate
        // any previous message pointed/owned to by self.inner.msg.
        self.inner.arena = arena;
        self.inner.msg = msg;
        Ok(())
      }
    }
  }

  // constant: optional message pilout.GlobalOperand.Constant
  pub fn r#constant(&self) -> crate::pilout::GlobalOperand_::ConstantView {
    let submsg = unsafe { pilout_GlobalOperand_constant(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::GlobalOperand_::ConstantView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::GlobalOperand_::ConstantView::new(::__pb::__internal::Private, field),
      }
  }

  // challenge: optional message pilout.GlobalOperand.Challenge
  pub fn r#challenge(&self) -> crate::pilout::GlobalOperand_::ChallengeView {
    let submsg = unsafe { pilout_GlobalOperand_challenge(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::GlobalOperand_::ChallengeView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::GlobalOperand_::ChallengeView::new(::__pb::__internal::Private, field),
      }
  }

  // proofValue: optional message pilout.GlobalOperand.ProofValue
  pub fn r#proofValue(&self) -> crate::pilout::GlobalOperand_::ProofValueView {
    let submsg = unsafe { pilout_GlobalOperand_proofValue(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::GlobalOperand_::ProofValueView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::GlobalOperand_::ProofValueView::new(::__pb::__internal::Private, field),
      }
  }

  // subproofValue: optional message pilout.GlobalOperand.SubproofValue
  pub fn r#subproofValue(&self) -> crate::pilout::GlobalOperand_::SubproofValueView {
    let submsg = unsafe { pilout_GlobalOperand_subproofValue(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::GlobalOperand_::SubproofValueView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::GlobalOperand_::SubproofValueView::new(::__pb::__internal::Private, field),
      }
  }

  // publicValue: optional message pilout.GlobalOperand.PublicValue
  pub fn r#publicValue(&self) -> crate::pilout::GlobalOperand_::PublicValueView {
    let submsg = unsafe { pilout_GlobalOperand_publicValue(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::GlobalOperand_::PublicValueView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::GlobalOperand_::PublicValueView::new(::__pb::__internal::Private, field),
      }
  }

  // publicTableAggregatedValue: optional message pilout.GlobalOperand.PublicTableAggregatedValue
  pub fn r#publicTableAggregatedValue(&self) -> crate::pilout::GlobalOperand_::PublicTableAggregatedValueView {
    let submsg = unsafe { pilout_GlobalOperand_publicTableAggregatedValue(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::GlobalOperand_::PublicTableAggregatedValueView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::GlobalOperand_::PublicTableAggregatedValueView::new(::__pb::__internal::Private, field),
      }
  }

  // publicTableColumn: optional message pilout.GlobalOperand.PublicTableColumn
  pub fn r#publicTableColumn(&self) -> crate::pilout::GlobalOperand_::PublicTableColumnView {
    let submsg = unsafe { pilout_GlobalOperand_publicTableColumn(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::GlobalOperand_::PublicTableColumnView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::GlobalOperand_::PublicTableColumnView::new(::__pb::__internal::Private, field),
      }
  }

  // expression: optional message pilout.GlobalOperand.Expression
  pub fn r#expression(&self) -> crate::pilout::GlobalOperand_::ExpressionView {
    let submsg = unsafe { pilout_GlobalOperand_expression(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::GlobalOperand_::ExpressionView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::GlobalOperand_::ExpressionView::new(::__pb::__internal::Private, field),
      }
  }


  pub fn r#operand(&self) -> GlobalOperand_::Operand {
    match unsafe { pilout_GlobalOperand_operand_case(self.inner.msg) } {
      _ => GlobalOperand_::Operand::not_set(std::marker::PhantomData)
    }
  }

  pub fn r#operand_mut(&mut self) -> GlobalOperand_::OperandMut {
    match unsafe { pilout_GlobalOperand_operand_case(self.inner.msg) } {
      _ => GlobalOperand_::OperandMut::not_set(std::marker::PhantomData)
    }
  }

}  // impl GlobalOperand

impl ::__std::ops::Drop for GlobalOperand {
  fn drop(&mut self) {
  }
}

extern "C" {
  fn pilout_GlobalOperand_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
  fn pilout_GlobalOperand_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
  fn pilout_GlobalOperand_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_GlobalOperand_constant(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_GlobalOperand_challenge(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_GlobalOperand_proofValue(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_GlobalOperand_subproofValue(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_GlobalOperand_publicValue(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_GlobalOperand_publicTableAggregatedValue(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_GlobalOperand_publicTableColumn(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_GlobalOperand_expression(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;


  fn pilout_GlobalOperand_operand_case(raw_msg: ::__pb::__internal::RawMessage) -> GlobalOperand_::OperandCase;

}  // extern "C" for GlobalOperand

#[allow(non_snake_case)]
pub mod GlobalOperand_ {
  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct Constant {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `Constant` does not provide shared mutation with its arena.
  // - `ConstantMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for Constant {}

  impl ::__pb::Proxied for Constant {
    type View<'a> = ConstantView<'a>;
    type Mut<'a> = ConstantMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct ConstantView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> ConstantView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
  }

  // SAFETY:
  // - `ConstantView` does not perform any mutation.
  // - While a `ConstantView` exists, a `ConstantMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `ConstantMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for ConstantView<'_> {}
  unsafe impl Send for ConstantView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for ConstantView<'a> {
    type Proxied = Constant;

    fn as_view(&self) -> ::__pb::View<'a, Constant> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Constant> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<Constant> for ConstantView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<Constant>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct ConstantMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `ConstantMut` does not perform any shared mutation.
  // - `ConstantMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for ConstantMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for ConstantMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, Constant> {
      ConstantMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, Constant> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for ConstantMut<'a> {
    type Proxied = Constant;
    fn as_view(&self) -> ::__pb::View<'_, Constant> {
      ConstantView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Constant> where 'a: 'shorter {
      ConstantView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl Constant {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_GlobalOperand_Constant_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_GlobalOperand_Constant_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_GlobalOperand_Constant_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // value: optional bytes
    pub fn r#value(&self) -> &[u8] {
      let view = unsafe { pilout_GlobalOperand_Constant_value(self.inner.msg).as_ref() };
      view
    }

    pub fn value_mut(&mut self) -> ::__pb::Mut<'_, [u8]> {
      static VTABLE: ::__pb::__internal::BytesMutVTable = unsafe {
        ::__pb::__internal::BytesMutVTable::new(
          ::__pb::__internal::Private,
          pilout_GlobalOperand_Constant_value,
          pilout_GlobalOperand_Constant_set_value,
        )
      };
      unsafe {
        <::__pb::Mut<[u8]>>::from_inner(
          ::__pb::__internal::Private,
          ::__pb::__internal::RawVTableMutator::new(
            ::__pb::__internal::Private,
            ::__pb::__runtime::MutatorMessageRef::new(
              ::__pb::__internal::Private, &mut self.inner),
            &VTABLE,
          )
        )
      }
    }


  }  // impl Constant

  impl ::__std::ops::Drop for Constant {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_GlobalOperand_Constant_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_GlobalOperand_Constant_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_GlobalOperand_Constant_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_GlobalOperand_Constant_value(raw_msg: ::__pb::__internal::RawMessage) -> ::__pb::__internal::PtrAndLen;
    fn pilout_GlobalOperand_Constant_set_value(raw_msg: ::__pb::__internal::RawMessage, val: ::__pb::__internal::PtrAndLen);
    fn pilout_GlobalOperand_Constant_clear_value(raw_msg: ::__pb::__internal::RawMessage);


  }  // extern "C" for Constant

  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct Challenge {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `Challenge` does not provide shared mutation with its arena.
  // - `ChallengeMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for Challenge {}

  impl ::__pb::Proxied for Challenge {
    type View<'a> = ChallengeView<'a>;
    type Mut<'a> = ChallengeMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct ChallengeView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> ChallengeView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
    pub fn r#stage(&self) -> u32 { unsafe {
      pilout_GlobalOperand_Challenge_stage(self.msg)
    } }

    pub fn r#idx(&self) -> u32 { unsafe {
      pilout_GlobalOperand_Challenge_idx(self.msg)
    } }

  }

  // SAFETY:
  // - `ChallengeView` does not perform any mutation.
  // - While a `ChallengeView` exists, a `ChallengeMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `ChallengeMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for ChallengeView<'_> {}
  unsafe impl Send for ChallengeView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for ChallengeView<'a> {
    type Proxied = Challenge;

    fn as_view(&self) -> ::__pb::View<'a, Challenge> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Challenge> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<Challenge> for ChallengeView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<Challenge>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct ChallengeMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `ChallengeMut` does not perform any shared mutation.
  // - `ChallengeMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for ChallengeMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for ChallengeMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, Challenge> {
      ChallengeMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, Challenge> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for ChallengeMut<'a> {
    type Proxied = Challenge;
    fn as_view(&self) -> ::__pb::View<'_, Challenge> {
      ChallengeView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Challenge> where 'a: 'shorter {
      ChallengeView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl Challenge {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_GlobalOperand_Challenge_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_GlobalOperand_Challenge_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_GlobalOperand_Challenge_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // stage: optional uint32
    pub fn r#stage(&self) -> u32 {
      unsafe { pilout_GlobalOperand_Challenge_stage(self.inner.msg) }
    }
    pub fn r#stage_mut(&mut self) -> ::__pb::PrimitiveMut<'_, u32> {
      static VTABLE: ::__pb::__internal::PrimitiveVTable<u32> =
        ::__pb::__internal::PrimitiveVTable::new(
          ::__pb::__internal::Private,
          pilout_GlobalOperand_Challenge_stage,
          pilout_GlobalOperand_Challenge_set_stage,
        );

        ::__pb::PrimitiveMut::from_inner(
          ::__pb::__internal::Private,
          unsafe {
            ::__pb::__internal::RawVTableMutator::new(
              ::__pb::__internal::Private,
              ::__pb::__runtime::MutatorMessageRef::new(
                ::__pb::__internal::Private, &mut self.inner
              ),
              &VTABLE,
            )
          },
        )
    }

    // idx: optional uint32
    pub fn r#idx(&self) -> u32 {
      unsafe { pilout_GlobalOperand_Challenge_idx(self.inner.msg) }
    }
    pub fn r#idx_mut(&mut self) -> ::__pb::PrimitiveMut<'_, u32> {
      static VTABLE: ::__pb::__internal::PrimitiveVTable<u32> =
        ::__pb::__internal::PrimitiveVTable::new(
          ::__pb::__internal::Private,
          pilout_GlobalOperand_Challenge_idx,
          pilout_GlobalOperand_Challenge_set_idx,
        );

        ::__pb::PrimitiveMut::from_inner(
          ::__pb::__internal::Private,
          unsafe {
            ::__pb::__internal::RawVTableMutator::new(
              ::__pb::__internal::Private,
              ::__pb::__runtime::MutatorMessageRef::new(
                ::__pb::__internal::Private, &mut self.inner
              ),
              &VTABLE,
            )
          },
        )
    }


  }  // impl Challenge

  impl ::__std::ops::Drop for Challenge {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_GlobalOperand_Challenge_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_GlobalOperand_Challenge_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_GlobalOperand_Challenge_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_GlobalOperand_Challenge_stage(raw_msg: ::__pb::__internal::RawMessage) -> u32;
    fn pilout_GlobalOperand_Challenge_set_stage(raw_msg: ::__pb::__internal::RawMessage, val: u32);
    fn pilout_GlobalOperand_Challenge_clear_stage(raw_msg: ::__pb::__internal::RawMessage);

    fn pilout_GlobalOperand_Challenge_idx(raw_msg: ::__pb::__internal::RawMessage) -> u32;
    fn pilout_GlobalOperand_Challenge_set_idx(raw_msg: ::__pb::__internal::RawMessage, val: u32);
    fn pilout_GlobalOperand_Challenge_clear_idx(raw_msg: ::__pb::__internal::RawMessage);


  }  // extern "C" for Challenge

  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct ProofValue {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `ProofValue` does not provide shared mutation with its arena.
  // - `ProofValueMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for ProofValue {}

  impl ::__pb::Proxied for ProofValue {
    type View<'a> = ProofValueView<'a>;
    type Mut<'a> = ProofValueMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct ProofValueView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> ProofValueView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
    pub fn r#idx(&self) -> u32 { unsafe {
      pilout_GlobalOperand_ProofValue_idx(self.msg)
    } }

  }

  // SAFETY:
  // - `ProofValueView` does not perform any mutation.
  // - While a `ProofValueView` exists, a `ProofValueMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `ProofValueMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for ProofValueView<'_> {}
  unsafe impl Send for ProofValueView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for ProofValueView<'a> {
    type Proxied = ProofValue;

    fn as_view(&self) -> ::__pb::View<'a, ProofValue> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, ProofValue> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<ProofValue> for ProofValueView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<ProofValue>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct ProofValueMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `ProofValueMut` does not perform any shared mutation.
  // - `ProofValueMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for ProofValueMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for ProofValueMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, ProofValue> {
      ProofValueMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, ProofValue> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for ProofValueMut<'a> {
    type Proxied = ProofValue;
    fn as_view(&self) -> ::__pb::View<'_, ProofValue> {
      ProofValueView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, ProofValue> where 'a: 'shorter {
      ProofValueView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl ProofValue {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_GlobalOperand_ProofValue_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_GlobalOperand_ProofValue_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_GlobalOperand_ProofValue_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // idx: optional uint32
    pub fn r#idx(&self) -> u32 {
      unsafe { pilout_GlobalOperand_ProofValue_idx(self.inner.msg) }
    }
    pub fn r#idx_mut(&mut self) -> ::__pb::PrimitiveMut<'_, u32> {
      static VTABLE: ::__pb::__internal::PrimitiveVTable<u32> =
        ::__pb::__internal::PrimitiveVTable::new(
          ::__pb::__internal::Private,
          pilout_GlobalOperand_ProofValue_idx,
          pilout_GlobalOperand_ProofValue_set_idx,
        );

        ::__pb::PrimitiveMut::from_inner(
          ::__pb::__internal::Private,
          unsafe {
            ::__pb::__internal::RawVTableMutator::new(
              ::__pb::__internal::Private,
              ::__pb::__runtime::MutatorMessageRef::new(
                ::__pb::__internal::Private, &mut self.inner
              ),
              &VTABLE,
            )
          },
        )
    }


  }  // impl ProofValue

  impl ::__std::ops::Drop for ProofValue {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_GlobalOperand_ProofValue_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_GlobalOperand_ProofValue_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_GlobalOperand_ProofValue_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_GlobalOperand_ProofValue_idx(raw_msg: ::__pb::__internal::RawMessage) -> u32;
    fn pilout_GlobalOperand_ProofValue_set_idx(raw_msg: ::__pb::__internal::RawMessage, val: u32);
    fn pilout_GlobalOperand_ProofValue_clear_idx(raw_msg: ::__pb::__internal::RawMessage);


  }  // extern "C" for ProofValue

  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct SubproofValue {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `SubproofValue` does not provide shared mutation with its arena.
  // - `SubproofValueMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for SubproofValue {}

  impl ::__pb::Proxied for SubproofValue {
    type View<'a> = SubproofValueView<'a>;
    type Mut<'a> = SubproofValueMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct SubproofValueView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> SubproofValueView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
    pub fn r#subproofId(&self) -> u32 { unsafe {
      pilout_GlobalOperand_SubproofValue_subproofId(self.msg)
    } }

    pub fn r#idx(&self) -> u32 { unsafe {
      pilout_GlobalOperand_SubproofValue_idx(self.msg)
    } }

  }

  // SAFETY:
  // - `SubproofValueView` does not perform any mutation.
  // - While a `SubproofValueView` exists, a `SubproofValueMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `SubproofValueMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for SubproofValueView<'_> {}
  unsafe impl Send for SubproofValueView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for SubproofValueView<'a> {
    type Proxied = SubproofValue;

    fn as_view(&self) -> ::__pb::View<'a, SubproofValue> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, SubproofValue> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<SubproofValue> for SubproofValueView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<SubproofValue>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct SubproofValueMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `SubproofValueMut` does not perform any shared mutation.
  // - `SubproofValueMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for SubproofValueMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for SubproofValueMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, SubproofValue> {
      SubproofValueMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, SubproofValue> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for SubproofValueMut<'a> {
    type Proxied = SubproofValue;
    fn as_view(&self) -> ::__pb::View<'_, SubproofValue> {
      SubproofValueView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, SubproofValue> where 'a: 'shorter {
      SubproofValueView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl SubproofValue {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_GlobalOperand_SubproofValue_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_GlobalOperand_SubproofValue_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_GlobalOperand_SubproofValue_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // subproofId: optional uint32
    pub fn r#subproofId(&self) -> u32 {
      unsafe { pilout_GlobalOperand_SubproofValue_subproofId(self.inner.msg) }
    }
    pub fn r#subproofId_mut(&mut self) -> ::__pb::PrimitiveMut<'_, u32> {
      static VTABLE: ::__pb::__internal::PrimitiveVTable<u32> =
        ::__pb::__internal::PrimitiveVTable::new(
          ::__pb::__internal::Private,
          pilout_GlobalOperand_SubproofValue_subproofId,
          pilout_GlobalOperand_SubproofValue_set_subproofId,
        );

        ::__pb::PrimitiveMut::from_inner(
          ::__pb::__internal::Private,
          unsafe {
            ::__pb::__internal::RawVTableMutator::new(
              ::__pb::__internal::Private,
              ::__pb::__runtime::MutatorMessageRef::new(
                ::__pb::__internal::Private, &mut self.inner
              ),
              &VTABLE,
            )
          },
        )
    }

    // idx: optional uint32
    pub fn r#idx(&self) -> u32 {
      unsafe { pilout_GlobalOperand_SubproofValue_idx(self.inner.msg) }
    }
    pub fn r#idx_mut(&mut self) -> ::__pb::PrimitiveMut<'_, u32> {
      static VTABLE: ::__pb::__internal::PrimitiveVTable<u32> =
        ::__pb::__internal::PrimitiveVTable::new(
          ::__pb::__internal::Private,
          pilout_GlobalOperand_SubproofValue_idx,
          pilout_GlobalOperand_SubproofValue_set_idx,
        );

        ::__pb::PrimitiveMut::from_inner(
          ::__pb::__internal::Private,
          unsafe {
            ::__pb::__internal::RawVTableMutator::new(
              ::__pb::__internal::Private,
              ::__pb::__runtime::MutatorMessageRef::new(
                ::__pb::__internal::Private, &mut self.inner
              ),
              &VTABLE,
            )
          },
        )
    }


  }  // impl SubproofValue

  impl ::__std::ops::Drop for SubproofValue {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_GlobalOperand_SubproofValue_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_GlobalOperand_SubproofValue_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_GlobalOperand_SubproofValue_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_GlobalOperand_SubproofValue_subproofId(raw_msg: ::__pb::__internal::RawMessage) -> u32;
    fn pilout_GlobalOperand_SubproofValue_set_subproofId(raw_msg: ::__pb::__internal::RawMessage, val: u32);
    fn pilout_GlobalOperand_SubproofValue_clear_subproofId(raw_msg: ::__pb::__internal::RawMessage);

    fn pilout_GlobalOperand_SubproofValue_idx(raw_msg: ::__pb::__internal::RawMessage) -> u32;
    fn pilout_GlobalOperand_SubproofValue_set_idx(raw_msg: ::__pb::__internal::RawMessage, val: u32);
    fn pilout_GlobalOperand_SubproofValue_clear_idx(raw_msg: ::__pb::__internal::RawMessage);


  }  // extern "C" for SubproofValue

  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct PublicValue {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `PublicValue` does not provide shared mutation with its arena.
  // - `PublicValueMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for PublicValue {}

  impl ::__pb::Proxied for PublicValue {
    type View<'a> = PublicValueView<'a>;
    type Mut<'a> = PublicValueMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct PublicValueView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> PublicValueView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
    pub fn r#idx(&self) -> u32 { unsafe {
      pilout_GlobalOperand_PublicValue_idx(self.msg)
    } }

  }

  // SAFETY:
  // - `PublicValueView` does not perform any mutation.
  // - While a `PublicValueView` exists, a `PublicValueMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `PublicValueMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for PublicValueView<'_> {}
  unsafe impl Send for PublicValueView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for PublicValueView<'a> {
    type Proxied = PublicValue;

    fn as_view(&self) -> ::__pb::View<'a, PublicValue> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, PublicValue> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<PublicValue> for PublicValueView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<PublicValue>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct PublicValueMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `PublicValueMut` does not perform any shared mutation.
  // - `PublicValueMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for PublicValueMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for PublicValueMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, PublicValue> {
      PublicValueMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, PublicValue> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for PublicValueMut<'a> {
    type Proxied = PublicValue;
    fn as_view(&self) -> ::__pb::View<'_, PublicValue> {
      PublicValueView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, PublicValue> where 'a: 'shorter {
      PublicValueView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl PublicValue {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_GlobalOperand_PublicValue_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_GlobalOperand_PublicValue_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_GlobalOperand_PublicValue_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // idx: optional uint32
    pub fn r#idx(&self) -> u32 {
      unsafe { pilout_GlobalOperand_PublicValue_idx(self.inner.msg) }
    }
    pub fn r#idx_mut(&mut self) -> ::__pb::PrimitiveMut<'_, u32> {
      static VTABLE: ::__pb::__internal::PrimitiveVTable<u32> =
        ::__pb::__internal::PrimitiveVTable::new(
          ::__pb::__internal::Private,
          pilout_GlobalOperand_PublicValue_idx,
          pilout_GlobalOperand_PublicValue_set_idx,
        );

        ::__pb::PrimitiveMut::from_inner(
          ::__pb::__internal::Private,
          unsafe {
            ::__pb::__internal::RawVTableMutator::new(
              ::__pb::__internal::Private,
              ::__pb::__runtime::MutatorMessageRef::new(
                ::__pb::__internal::Private, &mut self.inner
              ),
              &VTABLE,
            )
          },
        )
    }


  }  // impl PublicValue

  impl ::__std::ops::Drop for PublicValue {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_GlobalOperand_PublicValue_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_GlobalOperand_PublicValue_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_GlobalOperand_PublicValue_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_GlobalOperand_PublicValue_idx(raw_msg: ::__pb::__internal::RawMessage) -> u32;
    fn pilout_GlobalOperand_PublicValue_set_idx(raw_msg: ::__pb::__internal::RawMessage, val: u32);
    fn pilout_GlobalOperand_PublicValue_clear_idx(raw_msg: ::__pb::__internal::RawMessage);


  }  // extern "C" for PublicValue

  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct PublicTableAggregatedValue {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `PublicTableAggregatedValue` does not provide shared mutation with its arena.
  // - `PublicTableAggregatedValueMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for PublicTableAggregatedValue {}

  impl ::__pb::Proxied for PublicTableAggregatedValue {
    type View<'a> = PublicTableAggregatedValueView<'a>;
    type Mut<'a> = PublicTableAggregatedValueMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct PublicTableAggregatedValueView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> PublicTableAggregatedValueView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
    pub fn r#idx(&self) -> u32 { unsafe {
      pilout_GlobalOperand_PublicTableAggregatedValue_idx(self.msg)
    } }

  }

  // SAFETY:
  // - `PublicTableAggregatedValueView` does not perform any mutation.
  // - While a `PublicTableAggregatedValueView` exists, a `PublicTableAggregatedValueMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `PublicTableAggregatedValueMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for PublicTableAggregatedValueView<'_> {}
  unsafe impl Send for PublicTableAggregatedValueView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for PublicTableAggregatedValueView<'a> {
    type Proxied = PublicTableAggregatedValue;

    fn as_view(&self) -> ::__pb::View<'a, PublicTableAggregatedValue> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, PublicTableAggregatedValue> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<PublicTableAggregatedValue> for PublicTableAggregatedValueView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<PublicTableAggregatedValue>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct PublicTableAggregatedValueMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `PublicTableAggregatedValueMut` does not perform any shared mutation.
  // - `PublicTableAggregatedValueMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for PublicTableAggregatedValueMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for PublicTableAggregatedValueMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, PublicTableAggregatedValue> {
      PublicTableAggregatedValueMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, PublicTableAggregatedValue> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for PublicTableAggregatedValueMut<'a> {
    type Proxied = PublicTableAggregatedValue;
    fn as_view(&self) -> ::__pb::View<'_, PublicTableAggregatedValue> {
      PublicTableAggregatedValueView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, PublicTableAggregatedValue> where 'a: 'shorter {
      PublicTableAggregatedValueView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl PublicTableAggregatedValue {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_GlobalOperand_PublicTableAggregatedValue_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_GlobalOperand_PublicTableAggregatedValue_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_GlobalOperand_PublicTableAggregatedValue_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // idx: optional uint32
    pub fn r#idx(&self) -> u32 {
      unsafe { pilout_GlobalOperand_PublicTableAggregatedValue_idx(self.inner.msg) }
    }
    pub fn r#idx_mut(&mut self) -> ::__pb::PrimitiveMut<'_, u32> {
      static VTABLE: ::__pb::__internal::PrimitiveVTable<u32> =
        ::__pb::__internal::PrimitiveVTable::new(
          ::__pb::__internal::Private,
          pilout_GlobalOperand_PublicTableAggregatedValue_idx,
          pilout_GlobalOperand_PublicTableAggregatedValue_set_idx,
        );

        ::__pb::PrimitiveMut::from_inner(
          ::__pb::__internal::Private,
          unsafe {
            ::__pb::__internal::RawVTableMutator::new(
              ::__pb::__internal::Private,
              ::__pb::__runtime::MutatorMessageRef::new(
                ::__pb::__internal::Private, &mut self.inner
              ),
              &VTABLE,
            )
          },
        )
    }


  }  // impl PublicTableAggregatedValue

  impl ::__std::ops::Drop for PublicTableAggregatedValue {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_GlobalOperand_PublicTableAggregatedValue_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_GlobalOperand_PublicTableAggregatedValue_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_GlobalOperand_PublicTableAggregatedValue_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_GlobalOperand_PublicTableAggregatedValue_idx(raw_msg: ::__pb::__internal::RawMessage) -> u32;
    fn pilout_GlobalOperand_PublicTableAggregatedValue_set_idx(raw_msg: ::__pb::__internal::RawMessage, val: u32);
    fn pilout_GlobalOperand_PublicTableAggregatedValue_clear_idx(raw_msg: ::__pb::__internal::RawMessage);


  }  // extern "C" for PublicTableAggregatedValue

  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct PublicTableColumn {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `PublicTableColumn` does not provide shared mutation with its arena.
  // - `PublicTableColumnMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for PublicTableColumn {}

  impl ::__pb::Proxied for PublicTableColumn {
    type View<'a> = PublicTableColumnView<'a>;
    type Mut<'a> = PublicTableColumnMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct PublicTableColumnView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> PublicTableColumnView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
    pub fn r#idx(&self) -> u32 { unsafe {
      pilout_GlobalOperand_PublicTableColumn_idx(self.msg)
    } }

    pub fn r#colIdx(&self) -> u32 { unsafe {
      pilout_GlobalOperand_PublicTableColumn_colIdx(self.msg)
    } }

  }

  // SAFETY:
  // - `PublicTableColumnView` does not perform any mutation.
  // - While a `PublicTableColumnView` exists, a `PublicTableColumnMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `PublicTableColumnMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for PublicTableColumnView<'_> {}
  unsafe impl Send for PublicTableColumnView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for PublicTableColumnView<'a> {
    type Proxied = PublicTableColumn;

    fn as_view(&self) -> ::__pb::View<'a, PublicTableColumn> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, PublicTableColumn> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<PublicTableColumn> for PublicTableColumnView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<PublicTableColumn>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct PublicTableColumnMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `PublicTableColumnMut` does not perform any shared mutation.
  // - `PublicTableColumnMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for PublicTableColumnMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for PublicTableColumnMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, PublicTableColumn> {
      PublicTableColumnMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, PublicTableColumn> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for PublicTableColumnMut<'a> {
    type Proxied = PublicTableColumn;
    fn as_view(&self) -> ::__pb::View<'_, PublicTableColumn> {
      PublicTableColumnView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, PublicTableColumn> where 'a: 'shorter {
      PublicTableColumnView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl PublicTableColumn {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_GlobalOperand_PublicTableColumn_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_GlobalOperand_PublicTableColumn_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_GlobalOperand_PublicTableColumn_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // idx: optional uint32
    pub fn r#idx(&self) -> u32 {
      unsafe { pilout_GlobalOperand_PublicTableColumn_idx(self.inner.msg) }
    }
    pub fn r#idx_mut(&mut self) -> ::__pb::PrimitiveMut<'_, u32> {
      static VTABLE: ::__pb::__internal::PrimitiveVTable<u32> =
        ::__pb::__internal::PrimitiveVTable::new(
          ::__pb::__internal::Private,
          pilout_GlobalOperand_PublicTableColumn_idx,
          pilout_GlobalOperand_PublicTableColumn_set_idx,
        );

        ::__pb::PrimitiveMut::from_inner(
          ::__pb::__internal::Private,
          unsafe {
            ::__pb::__internal::RawVTableMutator::new(
              ::__pb::__internal::Private,
              ::__pb::__runtime::MutatorMessageRef::new(
                ::__pb::__internal::Private, &mut self.inner
              ),
              &VTABLE,
            )
          },
        )
    }

    // colIdx: optional uint32
    pub fn r#colIdx(&self) -> u32 {
      unsafe { pilout_GlobalOperand_PublicTableColumn_colIdx(self.inner.msg) }
    }
    pub fn r#colIdx_mut(&mut self) -> ::__pb::PrimitiveMut<'_, u32> {
      static VTABLE: ::__pb::__internal::PrimitiveVTable<u32> =
        ::__pb::__internal::PrimitiveVTable::new(
          ::__pb::__internal::Private,
          pilout_GlobalOperand_PublicTableColumn_colIdx,
          pilout_GlobalOperand_PublicTableColumn_set_colIdx,
        );

        ::__pb::PrimitiveMut::from_inner(
          ::__pb::__internal::Private,
          unsafe {
            ::__pb::__internal::RawVTableMutator::new(
              ::__pb::__internal::Private,
              ::__pb::__runtime::MutatorMessageRef::new(
                ::__pb::__internal::Private, &mut self.inner
              ),
              &VTABLE,
            )
          },
        )
    }


  }  // impl PublicTableColumn

  impl ::__std::ops::Drop for PublicTableColumn {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_GlobalOperand_PublicTableColumn_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_GlobalOperand_PublicTableColumn_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_GlobalOperand_PublicTableColumn_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_GlobalOperand_PublicTableColumn_idx(raw_msg: ::__pb::__internal::RawMessage) -> u32;
    fn pilout_GlobalOperand_PublicTableColumn_set_idx(raw_msg: ::__pb::__internal::RawMessage, val: u32);
    fn pilout_GlobalOperand_PublicTableColumn_clear_idx(raw_msg: ::__pb::__internal::RawMessage);

    fn pilout_GlobalOperand_PublicTableColumn_colIdx(raw_msg: ::__pb::__internal::RawMessage) -> u32;
    fn pilout_GlobalOperand_PublicTableColumn_set_colIdx(raw_msg: ::__pb::__internal::RawMessage, val: u32);
    fn pilout_GlobalOperand_PublicTableColumn_clear_colIdx(raw_msg: ::__pb::__internal::RawMessage);


  }  // extern "C" for PublicTableColumn

  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct Expression {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `Expression` does not provide shared mutation with its arena.
  // - `ExpressionMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for Expression {}

  impl ::__pb::Proxied for Expression {
    type View<'a> = ExpressionView<'a>;
    type Mut<'a> = ExpressionMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct ExpressionView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> ExpressionView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
    pub fn r#idx(&self) -> u32 { unsafe {
      pilout_GlobalOperand_Expression_idx(self.msg)
    } }

  }

  // SAFETY:
  // - `ExpressionView` does not perform any mutation.
  // - While a `ExpressionView` exists, a `ExpressionMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `ExpressionMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for ExpressionView<'_> {}
  unsafe impl Send for ExpressionView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for ExpressionView<'a> {
    type Proxied = Expression;

    fn as_view(&self) -> ::__pb::View<'a, Expression> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Expression> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<Expression> for ExpressionView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<Expression>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct ExpressionMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `ExpressionMut` does not perform any shared mutation.
  // - `ExpressionMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for ExpressionMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for ExpressionMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, Expression> {
      ExpressionMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, Expression> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for ExpressionMut<'a> {
    type Proxied = Expression;
    fn as_view(&self) -> ::__pb::View<'_, Expression> {
      ExpressionView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Expression> where 'a: 'shorter {
      ExpressionView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl Expression {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_GlobalOperand_Expression_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_GlobalOperand_Expression_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_GlobalOperand_Expression_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // idx: optional uint32
    pub fn r#idx(&self) -> u32 {
      unsafe { pilout_GlobalOperand_Expression_idx(self.inner.msg) }
    }
    pub fn r#idx_mut(&mut self) -> ::__pb::PrimitiveMut<'_, u32> {
      static VTABLE: ::__pb::__internal::PrimitiveVTable<u32> =
        ::__pb::__internal::PrimitiveVTable::new(
          ::__pb::__internal::Private,
          pilout_GlobalOperand_Expression_idx,
          pilout_GlobalOperand_Expression_set_idx,
        );

        ::__pb::PrimitiveMut::from_inner(
          ::__pb::__internal::Private,
          unsafe {
            ::__pb::__internal::RawVTableMutator::new(
              ::__pb::__internal::Private,
              ::__pb::__runtime::MutatorMessageRef::new(
                ::__pb::__internal::Private, &mut self.inner
              ),
              &VTABLE,
            )
          },
        )
    }


  }  // impl Expression

  impl ::__std::ops::Drop for Expression {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_GlobalOperand_Expression_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_GlobalOperand_Expression_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_GlobalOperand_Expression_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_GlobalOperand_Expression_idx(raw_msg: ::__pb::__internal::RawMessage) -> u32;
    fn pilout_GlobalOperand_Expression_set_idx(raw_msg: ::__pb::__internal::RawMessage, val: u32);
    fn pilout_GlobalOperand_Expression_clear_idx(raw_msg: ::__pb::__internal::RawMessage);


  }  // extern "C" for Expression


  #[non_exhaustive]
  #[derive(Debug)]
  #[allow(dead_code)]
  #[repr(isize)]
  pub enum Operand<'msg> {

    #[allow(non_camel_case_types)]
    not_set(std::marker::PhantomData<&'msg ()>) = 0
  }

  #[non_exhaustive]
  #[derive(Debug)]
  #[allow(dead_code)]
  #[repr(isize)]
  pub enum OperandMut<'msg> {

    #[allow(non_camel_case_types)]
    not_set(std::marker::PhantomData<&'msg ()>) = 0
  }
  #[repr(C)]
  #[derive(Debug, Copy, Clone, PartialEq, Eq)]
  pub(super) enum OperandCase {
    Constant = 1,
    Challenge = 2,
    ProofValue = 3,
    SubproofValue = 4,
    PublicValue = 5,
    PublicTableAggregatedValue = 6,
    PublicTableColumn = 7,
    Expression = 8,

    #[allow(non_camel_case_types)]
    not_set = 0
  }
}  // mod GlobalOperand_

#[allow(non_camel_case_types)]
// TODO: Implement support for debug redaction
#[derive(Debug)]
pub struct BasicAir {
  inner: ::__pb::__runtime::MessageInner
}

// SAFETY:
// - `BasicAir` does not provide shared mutation with its arena.
// - `BasicAirMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena that would conflict with
//   field access is impossible.
unsafe impl Sync for BasicAir {}

impl ::__pb::Proxied for BasicAir {
  type View<'a> = BasicAirView<'a>;
  type Mut<'a> = BasicAirMut<'a>;
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub struct BasicAirView<'a> {
  msg: ::__pb::__internal::RawMessage,
  _phantom: ::__std::marker::PhantomData<&'a ()>,
}

impl<'a> BasicAirView<'a> {
  #[doc(hidden)]
  pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
    Self { msg, _phantom: std::marker::PhantomData }
  }
  pub fn r#numRows(&self) -> u32 { unsafe {
    pilout_BasicAir_numRows(self.msg)
  } }

}

// SAFETY:
// - `BasicAirView` does not perform any mutation.
// - While a `BasicAirView` exists, a `BasicAirMut` can't exist to mutate
//   the arena that would conflict with field access.
// - `BasicAirMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for BasicAirView<'_> {}
unsafe impl Send for BasicAirView<'_> {}

impl<'a> ::__pb::ViewProxy<'a> for BasicAirView<'a> {
  type Proxied = BasicAir;

  fn as_view(&self) -> ::__pb::View<'a, BasicAir> {
    *self
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, BasicAir> where 'a: 'shorter {
    self
  }
}

impl<'a> ::__pb::SettableValue<BasicAir> for BasicAirView<'a> {
  fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<BasicAir>) {
    todo!()
  }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct BasicAirMut<'a> {
  inner: ::__pb::__runtime::MutatorMessageRef<'a>,
}

// SAFETY:
// - `BasicAirMut` does not perform any shared mutation.
// - `BasicAirMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for BasicAirMut<'_> {}

impl<'a> ::__pb::MutProxy<'a> for BasicAirMut<'a> {
  fn as_mut(&mut self) -> ::__pb::Mut<'_, BasicAir> {
    BasicAirMut { inner: self.inner }
  }
  fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, BasicAir> where 'a : 'shorter { self }
}

impl<'a> ::__pb::ViewProxy<'a> for BasicAirMut<'a> {
  type Proxied = BasicAir;
  fn as_view(&self) -> ::__pb::View<'_, BasicAir> {
    BasicAirView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, BasicAir> where 'a: 'shorter {
    BasicAirView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
}

impl BasicAir {
  pub fn new() -> Self {
    let arena = ::__pb::__runtime::Arena::new();
    Self {
      inner: ::__pb::__runtime::MessageInner {
        msg: unsafe { pilout_BasicAir_new(arena.raw()) },
        arena,
      }
    }
  }

  pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
    let arena = ::__pb::__runtime::Arena::new();
    let mut len = 0;
    unsafe {
      let data = pilout_BasicAir_serialize(self.inner.msg, arena.raw(), &mut len);
      ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
    }
  }
  pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
    let arena = ::__pb::__runtime::Arena::new();
    let msg = unsafe {
      pilout_BasicAir_parse(data.as_ptr(), data.len(), arena.raw())
    };

    match msg {
      None => Err(::__pb::ParseError),
      Some(msg) => {
        // This assignment causes self.arena to be dropped and to deallocate
        // any previous message pointed/owned to by self.inner.msg.
        self.inner.arena = arena;
        self.inner.msg = msg;
        Ok(())
      }
    }
  }

  // name: optional string
  pub fn r#name(&self) -> &::__pb::ProtoStr {
    let view = unsafe { pilout_BasicAir_name(self.inner.msg).as_ref() };
    // SAFETY: The runtime doesn't require ProtoStr to be UTF-8.
    unsafe { ::__pb::ProtoStr::from_utf8_unchecked(view) }
  }

  pub fn name_opt(&self) -> ::__pb::Optional<&::__pb::ProtoStr> {
    unsafe {
      let view = pilout_BasicAir_name(self.inner.msg).as_ref();
      ::__pb::Optional::new(
        // SAFETY: The runtime doesn't require ProtoStr to be UTF-8.
        unsafe { ::__pb::ProtoStr::from_utf8_unchecked(view) }
         ,
        pilout_BasicAir_has_name(self.inner.msg)
      )
    }
  }
  pub fn name_mut(&mut self) -> ::__pb::FieldEntry<'_, ::__pb::ProtoStr> {
    static VTABLE: ::__pb::__internal::BytesOptionalMutVTable = unsafe {
      ::__pb::__internal::BytesOptionalMutVTable::new(
        ::__pb::__internal::Private,
        pilout_BasicAir_name,
        pilout_BasicAir_set_name,
        pilout_BasicAir_clear_name,
        b"",
      )
    };
    let out = unsafe {
      let has = pilout_BasicAir_has_name(self.inner.msg);
      ::__pb::__internal::new_vtable_field_entry(
        ::__pb::__internal::Private,
        ::__pb::__runtime::MutatorMessageRef::new(
          ::__pb::__internal::Private, &mut self.inner),
        &VTABLE,
        has,
      )
    };
    ::__pb::ProtoStrMut::field_entry_from_bytes(
      ::__pb::__internal::Private, out
    )
  }

  // numRows: optional uint32
  pub fn r#numRows(&self) -> u32 {
    unsafe { pilout_BasicAir_numRows(self.inner.msg) }
  }
  pub fn r#numRows_opt(&self) -> ::__pb::Optional<u32> {
    if !unsafe { pilout_BasicAir_has_numRows(self.inner.msg) } {
      return ::__pb::Optional::Unset(<u32>::default());
    }
    let value = unsafe { pilout_BasicAir_numRows(self.inner.msg) };
    ::__pb::Optional::Set(value)
  }
  pub fn r#numRows_set(&mut self, val: Option<u32>) {
    match val {
      Some(val) => unsafe { pilout_BasicAir_set_numRows(self.inner.msg, val) },
      None => unsafe { pilout_BasicAir_clear_numRows(self.inner.msg) },
    }
  }

  // periodicCols: repeated message pilout.PeriodicCol
  // Unsupported! :(


  // fixedCols: repeated message pilout.FixedCol
  // Unsupported! :(


  // stageWidths: repeated uint32
  // Unsupported! :(


  // expressions: repeated message pilout.Expression
  // Unsupported! :(


  // constraints: repeated message pilout.Constraint
  // Unsupported! :(



}  // impl BasicAir

impl ::__std::ops::Drop for BasicAir {
  fn drop(&mut self) {
  }
}

extern "C" {
  fn pilout_BasicAir_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
  fn pilout_BasicAir_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
  fn pilout_BasicAir_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_BasicAir_has_name(raw_msg: ::__pb::__internal::RawMessage) -> bool;
  fn pilout_BasicAir_name(raw_msg: ::__pb::__internal::RawMessage) -> ::__pb::__internal::PtrAndLen;
  fn pilout_BasicAir_set_name(raw_msg: ::__pb::__internal::RawMessage, val: ::__pb::__internal::PtrAndLen);
  fn pilout_BasicAir_clear_name(raw_msg: ::__pb::__internal::RawMessage);

  fn pilout_BasicAir_has_numRows(raw_msg: ::__pb::__internal::RawMessage) -> bool;
  fn pilout_BasicAir_numRows(raw_msg: ::__pb::__internal::RawMessage) -> u32;
  fn pilout_BasicAir_set_numRows(raw_msg: ::__pb::__internal::RawMessage, val: u32);
  fn pilout_BasicAir_clear_numRows(raw_msg: ::__pb::__internal::RawMessage);







}  // extern "C" for BasicAir


#[allow(non_camel_case_types)]
// TODO: Implement support for debug redaction
#[derive(Debug)]
pub struct PeriodicCol {
  inner: ::__pb::__runtime::MessageInner
}

// SAFETY:
// - `PeriodicCol` does not provide shared mutation with its arena.
// - `PeriodicColMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena that would conflict with
//   field access is impossible.
unsafe impl Sync for PeriodicCol {}

impl ::__pb::Proxied for PeriodicCol {
  type View<'a> = PeriodicColView<'a>;
  type Mut<'a> = PeriodicColMut<'a>;
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub struct PeriodicColView<'a> {
  msg: ::__pb::__internal::RawMessage,
  _phantom: ::__std::marker::PhantomData<&'a ()>,
}

impl<'a> PeriodicColView<'a> {
  #[doc(hidden)]
  pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
    Self { msg, _phantom: std::marker::PhantomData }
  }
}

// SAFETY:
// - `PeriodicColView` does not perform any mutation.
// - While a `PeriodicColView` exists, a `PeriodicColMut` can't exist to mutate
//   the arena that would conflict with field access.
// - `PeriodicColMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for PeriodicColView<'_> {}
unsafe impl Send for PeriodicColView<'_> {}

impl<'a> ::__pb::ViewProxy<'a> for PeriodicColView<'a> {
  type Proxied = PeriodicCol;

  fn as_view(&self) -> ::__pb::View<'a, PeriodicCol> {
    *self
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, PeriodicCol> where 'a: 'shorter {
    self
  }
}

impl<'a> ::__pb::SettableValue<PeriodicCol> for PeriodicColView<'a> {
  fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<PeriodicCol>) {
    todo!()
  }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct PeriodicColMut<'a> {
  inner: ::__pb::__runtime::MutatorMessageRef<'a>,
}

// SAFETY:
// - `PeriodicColMut` does not perform any shared mutation.
// - `PeriodicColMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for PeriodicColMut<'_> {}

impl<'a> ::__pb::MutProxy<'a> for PeriodicColMut<'a> {
  fn as_mut(&mut self) -> ::__pb::Mut<'_, PeriodicCol> {
    PeriodicColMut { inner: self.inner }
  }
  fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, PeriodicCol> where 'a : 'shorter { self }
}

impl<'a> ::__pb::ViewProxy<'a> for PeriodicColMut<'a> {
  type Proxied = PeriodicCol;
  fn as_view(&self) -> ::__pb::View<'_, PeriodicCol> {
    PeriodicColView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, PeriodicCol> where 'a: 'shorter {
    PeriodicColView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
}

impl PeriodicCol {
  pub fn new() -> Self {
    let arena = ::__pb::__runtime::Arena::new();
    Self {
      inner: ::__pb::__runtime::MessageInner {
        msg: unsafe { pilout_PeriodicCol_new(arena.raw()) },
        arena,
      }
    }
  }

  pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
    let arena = ::__pb::__runtime::Arena::new();
    let mut len = 0;
    unsafe {
      let data = pilout_PeriodicCol_serialize(self.inner.msg, arena.raw(), &mut len);
      ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
    }
  }
  pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
    let arena = ::__pb::__runtime::Arena::new();
    let msg = unsafe {
      pilout_PeriodicCol_parse(data.as_ptr(), data.len(), arena.raw())
    };

    match msg {
      None => Err(::__pb::ParseError),
      Some(msg) => {
        // This assignment causes self.arena to be dropped and to deallocate
        // any previous message pointed/owned to by self.inner.msg.
        self.inner.arena = arena;
        self.inner.msg = msg;
        Ok(())
      }
    }
  }

  // values: repeated bytes
  // Unsupported! :(



}  // impl PeriodicCol

impl ::__std::ops::Drop for PeriodicCol {
  fn drop(&mut self) {
  }
}

extern "C" {
  fn pilout_PeriodicCol_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
  fn pilout_PeriodicCol_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
  fn pilout_PeriodicCol_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;



}  // extern "C" for PeriodicCol


#[allow(non_camel_case_types)]
// TODO: Implement support for debug redaction
#[derive(Debug)]
pub struct FixedCol {
  inner: ::__pb::__runtime::MessageInner
}

// SAFETY:
// - `FixedCol` does not provide shared mutation with its arena.
// - `FixedColMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena that would conflict with
//   field access is impossible.
unsafe impl Sync for FixedCol {}

impl ::__pb::Proxied for FixedCol {
  type View<'a> = FixedColView<'a>;
  type Mut<'a> = FixedColMut<'a>;
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub struct FixedColView<'a> {
  msg: ::__pb::__internal::RawMessage,
  _phantom: ::__std::marker::PhantomData<&'a ()>,
}

impl<'a> FixedColView<'a> {
  #[doc(hidden)]
  pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
    Self { msg, _phantom: std::marker::PhantomData }
  }
}

// SAFETY:
// - `FixedColView` does not perform any mutation.
// - While a `FixedColView` exists, a `FixedColMut` can't exist to mutate
//   the arena that would conflict with field access.
// - `FixedColMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for FixedColView<'_> {}
unsafe impl Send for FixedColView<'_> {}

impl<'a> ::__pb::ViewProxy<'a> for FixedColView<'a> {
  type Proxied = FixedCol;

  fn as_view(&self) -> ::__pb::View<'a, FixedCol> {
    *self
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, FixedCol> where 'a: 'shorter {
    self
  }
}

impl<'a> ::__pb::SettableValue<FixedCol> for FixedColView<'a> {
  fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<FixedCol>) {
    todo!()
  }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct FixedColMut<'a> {
  inner: ::__pb::__runtime::MutatorMessageRef<'a>,
}

// SAFETY:
// - `FixedColMut` does not perform any shared mutation.
// - `FixedColMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for FixedColMut<'_> {}

impl<'a> ::__pb::MutProxy<'a> for FixedColMut<'a> {
  fn as_mut(&mut self) -> ::__pb::Mut<'_, FixedCol> {
    FixedColMut { inner: self.inner }
  }
  fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, FixedCol> where 'a : 'shorter { self }
}

impl<'a> ::__pb::ViewProxy<'a> for FixedColMut<'a> {
  type Proxied = FixedCol;
  fn as_view(&self) -> ::__pb::View<'_, FixedCol> {
    FixedColView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, FixedCol> where 'a: 'shorter {
    FixedColView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
}

impl FixedCol {
  pub fn new() -> Self {
    let arena = ::__pb::__runtime::Arena::new();
    Self {
      inner: ::__pb::__runtime::MessageInner {
        msg: unsafe { pilout_FixedCol_new(arena.raw()) },
        arena,
      }
    }
  }

  pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
    let arena = ::__pb::__runtime::Arena::new();
    let mut len = 0;
    unsafe {
      let data = pilout_FixedCol_serialize(self.inner.msg, arena.raw(), &mut len);
      ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
    }
  }
  pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
    let arena = ::__pb::__runtime::Arena::new();
    let msg = unsafe {
      pilout_FixedCol_parse(data.as_ptr(), data.len(), arena.raw())
    };

    match msg {
      None => Err(::__pb::ParseError),
      Some(msg) => {
        // This assignment causes self.arena to be dropped and to deallocate
        // any previous message pointed/owned to by self.inner.msg.
        self.inner.arena = arena;
        self.inner.msg = msg;
        Ok(())
      }
    }
  }

  // values: repeated bytes
  // Unsupported! :(



}  // impl FixedCol

impl ::__std::ops::Drop for FixedCol {
  fn drop(&mut self) {
  }
}

extern "C" {
  fn pilout_FixedCol_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
  fn pilout_FixedCol_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
  fn pilout_FixedCol_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;



}  // extern "C" for FixedCol


#[allow(non_camel_case_types)]
// TODO: Implement support for debug redaction
#[derive(Debug)]
pub struct Constraint {
  inner: ::__pb::__runtime::MessageInner
}

// SAFETY:
// - `Constraint` does not provide shared mutation with its arena.
// - `ConstraintMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena that would conflict with
//   field access is impossible.
unsafe impl Sync for Constraint {}

impl ::__pb::Proxied for Constraint {
  type View<'a> = ConstraintView<'a>;
  type Mut<'a> = ConstraintMut<'a>;
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub struct ConstraintView<'a> {
  msg: ::__pb::__internal::RawMessage,
  _phantom: ::__std::marker::PhantomData<&'a ()>,
}

impl<'a> ConstraintView<'a> {
  #[doc(hidden)]
  pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
    Self { msg, _phantom: std::marker::PhantomData }
  }
}

// SAFETY:
// - `ConstraintView` does not perform any mutation.
// - While a `ConstraintView` exists, a `ConstraintMut` can't exist to mutate
//   the arena that would conflict with field access.
// - `ConstraintMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for ConstraintView<'_> {}
unsafe impl Send for ConstraintView<'_> {}

impl<'a> ::__pb::ViewProxy<'a> for ConstraintView<'a> {
  type Proxied = Constraint;

  fn as_view(&self) -> ::__pb::View<'a, Constraint> {
    *self
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Constraint> where 'a: 'shorter {
    self
  }
}

impl<'a> ::__pb::SettableValue<Constraint> for ConstraintView<'a> {
  fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<Constraint>) {
    todo!()
  }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct ConstraintMut<'a> {
  inner: ::__pb::__runtime::MutatorMessageRef<'a>,
}

// SAFETY:
// - `ConstraintMut` does not perform any shared mutation.
// - `ConstraintMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for ConstraintMut<'_> {}

impl<'a> ::__pb::MutProxy<'a> for ConstraintMut<'a> {
  fn as_mut(&mut self) -> ::__pb::Mut<'_, Constraint> {
    ConstraintMut { inner: self.inner }
  }
  fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, Constraint> where 'a : 'shorter { self }
}

impl<'a> ::__pb::ViewProxy<'a> for ConstraintMut<'a> {
  type Proxied = Constraint;
  fn as_view(&self) -> ::__pb::View<'_, Constraint> {
    ConstraintView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Constraint> where 'a: 'shorter {
    ConstraintView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
}

impl Constraint {
  pub fn new() -> Self {
    let arena = ::__pb::__runtime::Arena::new();
    Self {
      inner: ::__pb::__runtime::MessageInner {
        msg: unsafe { pilout_Constraint_new(arena.raw()) },
        arena,
      }
    }
  }

  pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
    let arena = ::__pb::__runtime::Arena::new();
    let mut len = 0;
    unsafe {
      let data = pilout_Constraint_serialize(self.inner.msg, arena.raw(), &mut len);
      ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
    }
  }
  pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
    let arena = ::__pb::__runtime::Arena::new();
    let msg = unsafe {
      pilout_Constraint_parse(data.as_ptr(), data.len(), arena.raw())
    };

    match msg {
      None => Err(::__pb::ParseError),
      Some(msg) => {
        // This assignment causes self.arena to be dropped and to deallocate
        // any previous message pointed/owned to by self.inner.msg.
        self.inner.arena = arena;
        self.inner.msg = msg;
        Ok(())
      }
    }
  }

  // firstRow: optional message pilout.Constraint.FirstRow
  pub fn r#firstRow(&self) -> crate::pilout::Constraint_::FirstRowView {
    let submsg = unsafe { pilout_Constraint_firstRow(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::Constraint_::FirstRowView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::Constraint_::FirstRowView::new(::__pb::__internal::Private, field),
      }
  }

  // lastRow: optional message pilout.Constraint.LastRow
  pub fn r#lastRow(&self) -> crate::pilout::Constraint_::LastRowView {
    let submsg = unsafe { pilout_Constraint_lastRow(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::Constraint_::LastRowView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::Constraint_::LastRowView::new(::__pb::__internal::Private, field),
      }
  }

  // everyRow: optional message pilout.Constraint.EveryRow
  pub fn r#everyRow(&self) -> crate::pilout::Constraint_::EveryRowView {
    let submsg = unsafe { pilout_Constraint_everyRow(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::Constraint_::EveryRowView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::Constraint_::EveryRowView::new(::__pb::__internal::Private, field),
      }
  }

  // everyFrame: optional message pilout.Constraint.EveryFrame
  pub fn r#everyFrame(&self) -> crate::pilout::Constraint_::EveryFrameView {
    let submsg = unsafe { pilout_Constraint_everyFrame(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::Constraint_::EveryFrameView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::Constraint_::EveryFrameView::new(::__pb::__internal::Private, field),
      }
  }


  pub fn r#constraint(&self) -> Constraint_::Constraint {
    match unsafe { pilout_Constraint_constraint_case(self.inner.msg) } {
      _ => Constraint_::Constraint::not_set(std::marker::PhantomData)
    }
  }

  pub fn r#constraint_mut(&mut self) -> Constraint_::ConstraintMut {
    match unsafe { pilout_Constraint_constraint_case(self.inner.msg) } {
      _ => Constraint_::ConstraintMut::not_set(std::marker::PhantomData)
    }
  }

}  // impl Constraint

impl ::__std::ops::Drop for Constraint {
  fn drop(&mut self) {
  }
}

extern "C" {
  fn pilout_Constraint_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
  fn pilout_Constraint_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
  fn pilout_Constraint_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_Constraint_firstRow(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_Constraint_lastRow(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_Constraint_everyRow(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_Constraint_everyFrame(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;


  fn pilout_Constraint_constraint_case(raw_msg: ::__pb::__internal::RawMessage) -> Constraint_::ConstraintCase;

}  // extern "C" for Constraint

#[allow(non_snake_case)]
pub mod Constraint_ {
  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct FirstRow {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `FirstRow` does not provide shared mutation with its arena.
  // - `FirstRowMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for FirstRow {}

  impl ::__pb::Proxied for FirstRow {
    type View<'a> = FirstRowView<'a>;
    type Mut<'a> = FirstRowMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct FirstRowView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> FirstRowView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
  }

  // SAFETY:
  // - `FirstRowView` does not perform any mutation.
  // - While a `FirstRowView` exists, a `FirstRowMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `FirstRowMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for FirstRowView<'_> {}
  unsafe impl Send for FirstRowView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for FirstRowView<'a> {
    type Proxied = FirstRow;

    fn as_view(&self) -> ::__pb::View<'a, FirstRow> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, FirstRow> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<FirstRow> for FirstRowView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<FirstRow>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct FirstRowMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `FirstRowMut` does not perform any shared mutation.
  // - `FirstRowMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for FirstRowMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for FirstRowMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, FirstRow> {
      FirstRowMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, FirstRow> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for FirstRowMut<'a> {
    type Proxied = FirstRow;
    fn as_view(&self) -> ::__pb::View<'_, FirstRow> {
      FirstRowView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, FirstRow> where 'a: 'shorter {
      FirstRowView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl FirstRow {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_Constraint_FirstRow_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_Constraint_FirstRow_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_Constraint_FirstRow_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // expressionIdx: optional message pilout.Operand.Expression
    pub fn r#expressionIdx(&self) -> crate::pilout::Operand_::ExpressionView {
      let submsg = unsafe { pilout_Constraint_FirstRow_expressionIdx(self.inner.msg) };
      // For upb, getters return null if the field is unset, so we need to
      // check for null and return the default instance manually. Note that
      // a null ptr received from upb manifests as Option::None
      match submsg {
          // TODO:(b/304357029)
          None => crate::pilout::Operand_::ExpressionView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
          Some(field) => crate::pilout::Operand_::ExpressionView::new(::__pb::__internal::Private, field),
        }
    }

    // debugLine: optional string
    pub fn r#debugLine(&self) -> &::__pb::ProtoStr {
      let view = unsafe { pilout_Constraint_FirstRow_debugLine(self.inner.msg).as_ref() };
      // SAFETY: The runtime doesn't require ProtoStr to be UTF-8.
      unsafe { ::__pb::ProtoStr::from_utf8_unchecked(view) }
    }

    pub fn debugLine_opt(&self) -> ::__pb::Optional<&::__pb::ProtoStr> {
      unsafe {
        let view = pilout_Constraint_FirstRow_debugLine(self.inner.msg).as_ref();
        ::__pb::Optional::new(
          // SAFETY: The runtime doesn't require ProtoStr to be UTF-8.
          unsafe { ::__pb::ProtoStr::from_utf8_unchecked(view) }
           ,
          pilout_Constraint_FirstRow_has_debugLine(self.inner.msg)
        )
      }
    }
    pub fn debugLine_mut(&mut self) -> ::__pb::FieldEntry<'_, ::__pb::ProtoStr> {
      static VTABLE: ::__pb::__internal::BytesOptionalMutVTable = unsafe {
        ::__pb::__internal::BytesOptionalMutVTable::new(
          ::__pb::__internal::Private,
          pilout_Constraint_FirstRow_debugLine,
          pilout_Constraint_FirstRow_set_debugLine,
          pilout_Constraint_FirstRow_clear_debugLine,
          b"",
        )
      };
      let out = unsafe {
        let has = pilout_Constraint_FirstRow_has_debugLine(self.inner.msg);
        ::__pb::__internal::new_vtable_field_entry(
          ::__pb::__internal::Private,
          ::__pb::__runtime::MutatorMessageRef::new(
            ::__pb::__internal::Private, &mut self.inner),
          &VTABLE,
          has,
        )
      };
      ::__pb::ProtoStrMut::field_entry_from_bytes(
        ::__pb::__internal::Private, out
      )
    }


  }  // impl FirstRow

  impl ::__std::ops::Drop for FirstRow {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_Constraint_FirstRow_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_Constraint_FirstRow_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_Constraint_FirstRow_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_Constraint_FirstRow_expressionIdx(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_Constraint_FirstRow_has_debugLine(raw_msg: ::__pb::__internal::RawMessage) -> bool;
    fn pilout_Constraint_FirstRow_debugLine(raw_msg: ::__pb::__internal::RawMessage) -> ::__pb::__internal::PtrAndLen;
    fn pilout_Constraint_FirstRow_set_debugLine(raw_msg: ::__pb::__internal::RawMessage, val: ::__pb::__internal::PtrAndLen);
    fn pilout_Constraint_FirstRow_clear_debugLine(raw_msg: ::__pb::__internal::RawMessage);


  }  // extern "C" for FirstRow

  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct LastRow {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `LastRow` does not provide shared mutation with its arena.
  // - `LastRowMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for LastRow {}

  impl ::__pb::Proxied for LastRow {
    type View<'a> = LastRowView<'a>;
    type Mut<'a> = LastRowMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct LastRowView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> LastRowView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
  }

  // SAFETY:
  // - `LastRowView` does not perform any mutation.
  // - While a `LastRowView` exists, a `LastRowMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `LastRowMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for LastRowView<'_> {}
  unsafe impl Send for LastRowView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for LastRowView<'a> {
    type Proxied = LastRow;

    fn as_view(&self) -> ::__pb::View<'a, LastRow> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, LastRow> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<LastRow> for LastRowView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<LastRow>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct LastRowMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `LastRowMut` does not perform any shared mutation.
  // - `LastRowMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for LastRowMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for LastRowMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, LastRow> {
      LastRowMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, LastRow> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for LastRowMut<'a> {
    type Proxied = LastRow;
    fn as_view(&self) -> ::__pb::View<'_, LastRow> {
      LastRowView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, LastRow> where 'a: 'shorter {
      LastRowView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl LastRow {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_Constraint_LastRow_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_Constraint_LastRow_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_Constraint_LastRow_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // expressionIdx: optional message pilout.Operand.Expression
    pub fn r#expressionIdx(&self) -> crate::pilout::Operand_::ExpressionView {
      let submsg = unsafe { pilout_Constraint_LastRow_expressionIdx(self.inner.msg) };
      // For upb, getters return null if the field is unset, so we need to
      // check for null and return the default instance manually. Note that
      // a null ptr received from upb manifests as Option::None
      match submsg {
          // TODO:(b/304357029)
          None => crate::pilout::Operand_::ExpressionView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
          Some(field) => crate::pilout::Operand_::ExpressionView::new(::__pb::__internal::Private, field),
        }
    }

    // debugLine: optional string
    pub fn r#debugLine(&self) -> &::__pb::ProtoStr {
      let view = unsafe { pilout_Constraint_LastRow_debugLine(self.inner.msg).as_ref() };
      // SAFETY: The runtime doesn't require ProtoStr to be UTF-8.
      unsafe { ::__pb::ProtoStr::from_utf8_unchecked(view) }
    }

    pub fn debugLine_opt(&self) -> ::__pb::Optional<&::__pb::ProtoStr> {
      unsafe {
        let view = pilout_Constraint_LastRow_debugLine(self.inner.msg).as_ref();
        ::__pb::Optional::new(
          // SAFETY: The runtime doesn't require ProtoStr to be UTF-8.
          unsafe { ::__pb::ProtoStr::from_utf8_unchecked(view) }
           ,
          pilout_Constraint_LastRow_has_debugLine(self.inner.msg)
        )
      }
    }
    pub fn debugLine_mut(&mut self) -> ::__pb::FieldEntry<'_, ::__pb::ProtoStr> {
      static VTABLE: ::__pb::__internal::BytesOptionalMutVTable = unsafe {
        ::__pb::__internal::BytesOptionalMutVTable::new(
          ::__pb::__internal::Private,
          pilout_Constraint_LastRow_debugLine,
          pilout_Constraint_LastRow_set_debugLine,
          pilout_Constraint_LastRow_clear_debugLine,
          b"",
        )
      };
      let out = unsafe {
        let has = pilout_Constraint_LastRow_has_debugLine(self.inner.msg);
        ::__pb::__internal::new_vtable_field_entry(
          ::__pb::__internal::Private,
          ::__pb::__runtime::MutatorMessageRef::new(
            ::__pb::__internal::Private, &mut self.inner),
          &VTABLE,
          has,
        )
      };
      ::__pb::ProtoStrMut::field_entry_from_bytes(
        ::__pb::__internal::Private, out
      )
    }


  }  // impl LastRow

  impl ::__std::ops::Drop for LastRow {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_Constraint_LastRow_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_Constraint_LastRow_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_Constraint_LastRow_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_Constraint_LastRow_expressionIdx(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_Constraint_LastRow_has_debugLine(raw_msg: ::__pb::__internal::RawMessage) -> bool;
    fn pilout_Constraint_LastRow_debugLine(raw_msg: ::__pb::__internal::RawMessage) -> ::__pb::__internal::PtrAndLen;
    fn pilout_Constraint_LastRow_set_debugLine(raw_msg: ::__pb::__internal::RawMessage, val: ::__pb::__internal::PtrAndLen);
    fn pilout_Constraint_LastRow_clear_debugLine(raw_msg: ::__pb::__internal::RawMessage);


  }  // extern "C" for LastRow

  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct EveryRow {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `EveryRow` does not provide shared mutation with its arena.
  // - `EveryRowMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for EveryRow {}

  impl ::__pb::Proxied for EveryRow {
    type View<'a> = EveryRowView<'a>;
    type Mut<'a> = EveryRowMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct EveryRowView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> EveryRowView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
  }

  // SAFETY:
  // - `EveryRowView` does not perform any mutation.
  // - While a `EveryRowView` exists, a `EveryRowMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `EveryRowMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for EveryRowView<'_> {}
  unsafe impl Send for EveryRowView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for EveryRowView<'a> {
    type Proxied = EveryRow;

    fn as_view(&self) -> ::__pb::View<'a, EveryRow> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, EveryRow> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<EveryRow> for EveryRowView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<EveryRow>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct EveryRowMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `EveryRowMut` does not perform any shared mutation.
  // - `EveryRowMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for EveryRowMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for EveryRowMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, EveryRow> {
      EveryRowMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, EveryRow> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for EveryRowMut<'a> {
    type Proxied = EveryRow;
    fn as_view(&self) -> ::__pb::View<'_, EveryRow> {
      EveryRowView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, EveryRow> where 'a: 'shorter {
      EveryRowView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl EveryRow {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_Constraint_EveryRow_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_Constraint_EveryRow_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_Constraint_EveryRow_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // expressionIdx: optional message pilout.Operand.Expression
    pub fn r#expressionIdx(&self) -> crate::pilout::Operand_::ExpressionView {
      let submsg = unsafe { pilout_Constraint_EveryRow_expressionIdx(self.inner.msg) };
      // For upb, getters return null if the field is unset, so we need to
      // check for null and return the default instance manually. Note that
      // a null ptr received from upb manifests as Option::None
      match submsg {
          // TODO:(b/304357029)
          None => crate::pilout::Operand_::ExpressionView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
          Some(field) => crate::pilout::Operand_::ExpressionView::new(::__pb::__internal::Private, field),
        }
    }

    // debugLine: optional string
    pub fn r#debugLine(&self) -> &::__pb::ProtoStr {
      let view = unsafe { pilout_Constraint_EveryRow_debugLine(self.inner.msg).as_ref() };
      // SAFETY: The runtime doesn't require ProtoStr to be UTF-8.
      unsafe { ::__pb::ProtoStr::from_utf8_unchecked(view) }
    }

    pub fn debugLine_opt(&self) -> ::__pb::Optional<&::__pb::ProtoStr> {
      unsafe {
        let view = pilout_Constraint_EveryRow_debugLine(self.inner.msg).as_ref();
        ::__pb::Optional::new(
          // SAFETY: The runtime doesn't require ProtoStr to be UTF-8.
          unsafe { ::__pb::ProtoStr::from_utf8_unchecked(view) }
           ,
          pilout_Constraint_EveryRow_has_debugLine(self.inner.msg)
        )
      }
    }
    pub fn debugLine_mut(&mut self) -> ::__pb::FieldEntry<'_, ::__pb::ProtoStr> {
      static VTABLE: ::__pb::__internal::BytesOptionalMutVTable = unsafe {
        ::__pb::__internal::BytesOptionalMutVTable::new(
          ::__pb::__internal::Private,
          pilout_Constraint_EveryRow_debugLine,
          pilout_Constraint_EveryRow_set_debugLine,
          pilout_Constraint_EveryRow_clear_debugLine,
          b"",
        )
      };
      let out = unsafe {
        let has = pilout_Constraint_EveryRow_has_debugLine(self.inner.msg);
        ::__pb::__internal::new_vtable_field_entry(
          ::__pb::__internal::Private,
          ::__pb::__runtime::MutatorMessageRef::new(
            ::__pb::__internal::Private, &mut self.inner),
          &VTABLE,
          has,
        )
      };
      ::__pb::ProtoStrMut::field_entry_from_bytes(
        ::__pb::__internal::Private, out
      )
    }


  }  // impl EveryRow

  impl ::__std::ops::Drop for EveryRow {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_Constraint_EveryRow_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_Constraint_EveryRow_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_Constraint_EveryRow_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_Constraint_EveryRow_expressionIdx(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_Constraint_EveryRow_has_debugLine(raw_msg: ::__pb::__internal::RawMessage) -> bool;
    fn pilout_Constraint_EveryRow_debugLine(raw_msg: ::__pb::__internal::RawMessage) -> ::__pb::__internal::PtrAndLen;
    fn pilout_Constraint_EveryRow_set_debugLine(raw_msg: ::__pb::__internal::RawMessage, val: ::__pb::__internal::PtrAndLen);
    fn pilout_Constraint_EveryRow_clear_debugLine(raw_msg: ::__pb::__internal::RawMessage);


  }  // extern "C" for EveryRow

  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct EveryFrame {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `EveryFrame` does not provide shared mutation with its arena.
  // - `EveryFrameMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for EveryFrame {}

  impl ::__pb::Proxied for EveryFrame {
    type View<'a> = EveryFrameView<'a>;
    type Mut<'a> = EveryFrameMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct EveryFrameView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> EveryFrameView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
    pub fn r#offsetMin(&self) -> u32 { unsafe {
      pilout_Constraint_EveryFrame_offsetMin(self.msg)
    } }

    pub fn r#offsetMax(&self) -> u32 { unsafe {
      pilout_Constraint_EveryFrame_offsetMax(self.msg)
    } }

  }

  // SAFETY:
  // - `EveryFrameView` does not perform any mutation.
  // - While a `EveryFrameView` exists, a `EveryFrameMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `EveryFrameMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for EveryFrameView<'_> {}
  unsafe impl Send for EveryFrameView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for EveryFrameView<'a> {
    type Proxied = EveryFrame;

    fn as_view(&self) -> ::__pb::View<'a, EveryFrame> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, EveryFrame> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<EveryFrame> for EveryFrameView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<EveryFrame>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct EveryFrameMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `EveryFrameMut` does not perform any shared mutation.
  // - `EveryFrameMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for EveryFrameMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for EveryFrameMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, EveryFrame> {
      EveryFrameMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, EveryFrame> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for EveryFrameMut<'a> {
    type Proxied = EveryFrame;
    fn as_view(&self) -> ::__pb::View<'_, EveryFrame> {
      EveryFrameView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, EveryFrame> where 'a: 'shorter {
      EveryFrameView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl EveryFrame {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_Constraint_EveryFrame_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_Constraint_EveryFrame_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_Constraint_EveryFrame_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // expressionIdx: optional message pilout.Operand.Expression
    pub fn r#expressionIdx(&self) -> crate::pilout::Operand_::ExpressionView {
      let submsg = unsafe { pilout_Constraint_EveryFrame_expressionIdx(self.inner.msg) };
      // For upb, getters return null if the field is unset, so we need to
      // check for null and return the default instance manually. Note that
      // a null ptr received from upb manifests as Option::None
      match submsg {
          // TODO:(b/304357029)
          None => crate::pilout::Operand_::ExpressionView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
          Some(field) => crate::pilout::Operand_::ExpressionView::new(::__pb::__internal::Private, field),
        }
    }

    // offsetMin: optional uint32
    pub fn r#offsetMin(&self) -> u32 {
      unsafe { pilout_Constraint_EveryFrame_offsetMin(self.inner.msg) }
    }
    pub fn r#offsetMin_mut(&mut self) -> ::__pb::PrimitiveMut<'_, u32> {
      static VTABLE: ::__pb::__internal::PrimitiveVTable<u32> =
        ::__pb::__internal::PrimitiveVTable::new(
          ::__pb::__internal::Private,
          pilout_Constraint_EveryFrame_offsetMin,
          pilout_Constraint_EveryFrame_set_offsetMin,
        );

        ::__pb::PrimitiveMut::from_inner(
          ::__pb::__internal::Private,
          unsafe {
            ::__pb::__internal::RawVTableMutator::new(
              ::__pb::__internal::Private,
              ::__pb::__runtime::MutatorMessageRef::new(
                ::__pb::__internal::Private, &mut self.inner
              ),
              &VTABLE,
            )
          },
        )
    }

    // offsetMax: optional uint32
    pub fn r#offsetMax(&self) -> u32 {
      unsafe { pilout_Constraint_EveryFrame_offsetMax(self.inner.msg) }
    }
    pub fn r#offsetMax_mut(&mut self) -> ::__pb::PrimitiveMut<'_, u32> {
      static VTABLE: ::__pb::__internal::PrimitiveVTable<u32> =
        ::__pb::__internal::PrimitiveVTable::new(
          ::__pb::__internal::Private,
          pilout_Constraint_EveryFrame_offsetMax,
          pilout_Constraint_EveryFrame_set_offsetMax,
        );

        ::__pb::PrimitiveMut::from_inner(
          ::__pb::__internal::Private,
          unsafe {
            ::__pb::__internal::RawVTableMutator::new(
              ::__pb::__internal::Private,
              ::__pb::__runtime::MutatorMessageRef::new(
                ::__pb::__internal::Private, &mut self.inner
              ),
              &VTABLE,
            )
          },
        )
    }

    // debugLine: optional string
    pub fn r#debugLine(&self) -> &::__pb::ProtoStr {
      let view = unsafe { pilout_Constraint_EveryFrame_debugLine(self.inner.msg).as_ref() };
      // SAFETY: The runtime doesn't require ProtoStr to be UTF-8.
      unsafe { ::__pb::ProtoStr::from_utf8_unchecked(view) }
    }

    pub fn debugLine_opt(&self) -> ::__pb::Optional<&::__pb::ProtoStr> {
      unsafe {
        let view = pilout_Constraint_EveryFrame_debugLine(self.inner.msg).as_ref();
        ::__pb::Optional::new(
          // SAFETY: The runtime doesn't require ProtoStr to be UTF-8.
          unsafe { ::__pb::ProtoStr::from_utf8_unchecked(view) }
           ,
          pilout_Constraint_EveryFrame_has_debugLine(self.inner.msg)
        )
      }
    }
    pub fn debugLine_mut(&mut self) -> ::__pb::FieldEntry<'_, ::__pb::ProtoStr> {
      static VTABLE: ::__pb::__internal::BytesOptionalMutVTable = unsafe {
        ::__pb::__internal::BytesOptionalMutVTable::new(
          ::__pb::__internal::Private,
          pilout_Constraint_EveryFrame_debugLine,
          pilout_Constraint_EveryFrame_set_debugLine,
          pilout_Constraint_EveryFrame_clear_debugLine,
          b"",
        )
      };
      let out = unsafe {
        let has = pilout_Constraint_EveryFrame_has_debugLine(self.inner.msg);
        ::__pb::__internal::new_vtable_field_entry(
          ::__pb::__internal::Private,
          ::__pb::__runtime::MutatorMessageRef::new(
            ::__pb::__internal::Private, &mut self.inner),
          &VTABLE,
          has,
        )
      };
      ::__pb::ProtoStrMut::field_entry_from_bytes(
        ::__pb::__internal::Private, out
      )
    }


  }  // impl EveryFrame

  impl ::__std::ops::Drop for EveryFrame {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_Constraint_EveryFrame_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_Constraint_EveryFrame_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_Constraint_EveryFrame_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_Constraint_EveryFrame_expressionIdx(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_Constraint_EveryFrame_offsetMin(raw_msg: ::__pb::__internal::RawMessage) -> u32;
    fn pilout_Constraint_EveryFrame_set_offsetMin(raw_msg: ::__pb::__internal::RawMessage, val: u32);
    fn pilout_Constraint_EveryFrame_clear_offsetMin(raw_msg: ::__pb::__internal::RawMessage);

    fn pilout_Constraint_EveryFrame_offsetMax(raw_msg: ::__pb::__internal::RawMessage) -> u32;
    fn pilout_Constraint_EveryFrame_set_offsetMax(raw_msg: ::__pb::__internal::RawMessage, val: u32);
    fn pilout_Constraint_EveryFrame_clear_offsetMax(raw_msg: ::__pb::__internal::RawMessage);

    fn pilout_Constraint_EveryFrame_has_debugLine(raw_msg: ::__pb::__internal::RawMessage) -> bool;
    fn pilout_Constraint_EveryFrame_debugLine(raw_msg: ::__pb::__internal::RawMessage) -> ::__pb::__internal::PtrAndLen;
    fn pilout_Constraint_EveryFrame_set_debugLine(raw_msg: ::__pb::__internal::RawMessage, val: ::__pb::__internal::PtrAndLen);
    fn pilout_Constraint_EveryFrame_clear_debugLine(raw_msg: ::__pb::__internal::RawMessage);


  }  // extern "C" for EveryFrame


  #[non_exhaustive]
  #[derive(Debug)]
  #[allow(dead_code)]
  #[repr(isize)]
  pub enum Constraint<'msg> {

    #[allow(non_camel_case_types)]
    not_set(std::marker::PhantomData<&'msg ()>) = 0
  }

  #[non_exhaustive]
  #[derive(Debug)]
  #[allow(dead_code)]
  #[repr(isize)]
  pub enum ConstraintMut<'msg> {

    #[allow(non_camel_case_types)]
    not_set(std::marker::PhantomData<&'msg ()>) = 0
  }
  #[repr(C)]
  #[derive(Debug, Copy, Clone, PartialEq, Eq)]
  pub(super) enum ConstraintCase {
    FirstRow = 1,
    LastRow = 2,
    EveryRow = 3,
    EveryFrame = 4,

    #[allow(non_camel_case_types)]
    not_set = 0
  }
}  // mod Constraint_

#[allow(non_camel_case_types)]
// TODO: Implement support for debug redaction
#[derive(Debug)]
pub struct Operand {
  inner: ::__pb::__runtime::MessageInner
}

// SAFETY:
// - `Operand` does not provide shared mutation with its arena.
// - `OperandMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena that would conflict with
//   field access is impossible.
unsafe impl Sync for Operand {}

impl ::__pb::Proxied for Operand {
  type View<'a> = OperandView<'a>;
  type Mut<'a> = OperandMut<'a>;
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub struct OperandView<'a> {
  msg: ::__pb::__internal::RawMessage,
  _phantom: ::__std::marker::PhantomData<&'a ()>,
}

impl<'a> OperandView<'a> {
  #[doc(hidden)]
  pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
    Self { msg, _phantom: std::marker::PhantomData }
  }
}

// SAFETY:
// - `OperandView` does not perform any mutation.
// - While a `OperandView` exists, a `OperandMut` can't exist to mutate
//   the arena that would conflict with field access.
// - `OperandMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for OperandView<'_> {}
unsafe impl Send for OperandView<'_> {}

impl<'a> ::__pb::ViewProxy<'a> for OperandView<'a> {
  type Proxied = Operand;

  fn as_view(&self) -> ::__pb::View<'a, Operand> {
    *self
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Operand> where 'a: 'shorter {
    self
  }
}

impl<'a> ::__pb::SettableValue<Operand> for OperandView<'a> {
  fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<Operand>) {
    todo!()
  }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct OperandMut<'a> {
  inner: ::__pb::__runtime::MutatorMessageRef<'a>,
}

// SAFETY:
// - `OperandMut` does not perform any shared mutation.
// - `OperandMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for OperandMut<'_> {}

impl<'a> ::__pb::MutProxy<'a> for OperandMut<'a> {
  fn as_mut(&mut self) -> ::__pb::Mut<'_, Operand> {
    OperandMut { inner: self.inner }
  }
  fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, Operand> where 'a : 'shorter { self }
}

impl<'a> ::__pb::ViewProxy<'a> for OperandMut<'a> {
  type Proxied = Operand;
  fn as_view(&self) -> ::__pb::View<'_, Operand> {
    OperandView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Operand> where 'a: 'shorter {
    OperandView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
}

impl Operand {
  pub fn new() -> Self {
    let arena = ::__pb::__runtime::Arena::new();
    Self {
      inner: ::__pb::__runtime::MessageInner {
        msg: unsafe { pilout_Operand_new(arena.raw()) },
        arena,
      }
    }
  }

  pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
    let arena = ::__pb::__runtime::Arena::new();
    let mut len = 0;
    unsafe {
      let data = pilout_Operand_serialize(self.inner.msg, arena.raw(), &mut len);
      ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
    }
  }
  pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
    let arena = ::__pb::__runtime::Arena::new();
    let msg = unsafe {
      pilout_Operand_parse(data.as_ptr(), data.len(), arena.raw())
    };

    match msg {
      None => Err(::__pb::ParseError),
      Some(msg) => {
        // This assignment causes self.arena to be dropped and to deallocate
        // any previous message pointed/owned to by self.inner.msg.
        self.inner.arena = arena;
        self.inner.msg = msg;
        Ok(())
      }
    }
  }

  // constant: optional message pilout.Operand.Constant
  pub fn r#constant(&self) -> crate::pilout::Operand_::ConstantView {
    let submsg = unsafe { pilout_Operand_constant(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::Operand_::ConstantView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::Operand_::ConstantView::new(::__pb::__internal::Private, field),
      }
  }

  // challenge: optional message pilout.Operand.Challenge
  pub fn r#challenge(&self) -> crate::pilout::Operand_::ChallengeView {
    let submsg = unsafe { pilout_Operand_challenge(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::Operand_::ChallengeView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::Operand_::ChallengeView::new(::__pb::__internal::Private, field),
      }
  }

  // proofValue: optional message pilout.Operand.ProofValue
  pub fn r#proofValue(&self) -> crate::pilout::Operand_::ProofValueView {
    let submsg = unsafe { pilout_Operand_proofValue(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::Operand_::ProofValueView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::Operand_::ProofValueView::new(::__pb::__internal::Private, field),
      }
  }

  // subproofValue: optional message pilout.Operand.SubproofValue
  pub fn r#subproofValue(&self) -> crate::pilout::Operand_::SubproofValueView {
    let submsg = unsafe { pilout_Operand_subproofValue(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::Operand_::SubproofValueView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::Operand_::SubproofValueView::new(::__pb::__internal::Private, field),
      }
  }

  // publicValue: optional message pilout.Operand.PublicValue
  pub fn r#publicValue(&self) -> crate::pilout::Operand_::PublicValueView {
    let submsg = unsafe { pilout_Operand_publicValue(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::Operand_::PublicValueView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::Operand_::PublicValueView::new(::__pb::__internal::Private, field),
      }
  }

  // periodicCol: optional message pilout.Operand.PeriodicCol
  pub fn r#periodicCol(&self) -> crate::pilout::Operand_::PeriodicColView {
    let submsg = unsafe { pilout_Operand_periodicCol(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::Operand_::PeriodicColView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::Operand_::PeriodicColView::new(::__pb::__internal::Private, field),
      }
  }

  // fixedCol: optional message pilout.Operand.FixedCol
  pub fn r#fixedCol(&self) -> crate::pilout::Operand_::FixedColView {
    let submsg = unsafe { pilout_Operand_fixedCol(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::Operand_::FixedColView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::Operand_::FixedColView::new(::__pb::__internal::Private, field),
      }
  }

  // witnessCol: optional message pilout.Operand.WitnessCol
  pub fn r#witnessCol(&self) -> crate::pilout::Operand_::WitnessColView {
    let submsg = unsafe { pilout_Operand_witnessCol(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::Operand_::WitnessColView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::Operand_::WitnessColView::new(::__pb::__internal::Private, field),
      }
  }

  // expression: optional message pilout.Operand.Expression
  pub fn r#expression(&self) -> crate::pilout::Operand_::ExpressionView {
    let submsg = unsafe { pilout_Operand_expression(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::Operand_::ExpressionView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::Operand_::ExpressionView::new(::__pb::__internal::Private, field),
      }
  }


  pub fn r#operand(&self) -> Operand_::Operand {
    match unsafe { pilout_Operand_operand_case(self.inner.msg) } {
      _ => Operand_::Operand::not_set(std::marker::PhantomData)
    }
  }

  pub fn r#operand_mut(&mut self) -> Operand_::OperandMut {
    match unsafe { pilout_Operand_operand_case(self.inner.msg) } {
      _ => Operand_::OperandMut::not_set(std::marker::PhantomData)
    }
  }

}  // impl Operand

impl ::__std::ops::Drop for Operand {
  fn drop(&mut self) {
  }
}

extern "C" {
  fn pilout_Operand_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
  fn pilout_Operand_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
  fn pilout_Operand_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_Operand_constant(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_Operand_challenge(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_Operand_proofValue(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_Operand_subproofValue(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_Operand_publicValue(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_Operand_periodicCol(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_Operand_fixedCol(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_Operand_witnessCol(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_Operand_expression(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;


  fn pilout_Operand_operand_case(raw_msg: ::__pb::__internal::RawMessage) -> Operand_::OperandCase;

}  // extern "C" for Operand

#[allow(non_snake_case)]
pub mod Operand_ {
  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct Constant {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `Constant` does not provide shared mutation with its arena.
  // - `ConstantMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for Constant {}

  impl ::__pb::Proxied for Constant {
    type View<'a> = ConstantView<'a>;
    type Mut<'a> = ConstantMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct ConstantView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> ConstantView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
  }

  // SAFETY:
  // - `ConstantView` does not perform any mutation.
  // - While a `ConstantView` exists, a `ConstantMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `ConstantMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for ConstantView<'_> {}
  unsafe impl Send for ConstantView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for ConstantView<'a> {
    type Proxied = Constant;

    fn as_view(&self) -> ::__pb::View<'a, Constant> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Constant> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<Constant> for ConstantView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<Constant>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct ConstantMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `ConstantMut` does not perform any shared mutation.
  // - `ConstantMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for ConstantMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for ConstantMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, Constant> {
      ConstantMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, Constant> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for ConstantMut<'a> {
    type Proxied = Constant;
    fn as_view(&self) -> ::__pb::View<'_, Constant> {
      ConstantView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Constant> where 'a: 'shorter {
      ConstantView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl Constant {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_Operand_Constant_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_Operand_Constant_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_Operand_Constant_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // value: optional bytes
    pub fn r#value(&self) -> &[u8] {
      let view = unsafe { pilout_Operand_Constant_value(self.inner.msg).as_ref() };
      view
    }

    pub fn value_mut(&mut self) -> ::__pb::Mut<'_, [u8]> {
      static VTABLE: ::__pb::__internal::BytesMutVTable = unsafe {
        ::__pb::__internal::BytesMutVTable::new(
          ::__pb::__internal::Private,
          pilout_Operand_Constant_value,
          pilout_Operand_Constant_set_value,
        )
      };
      unsafe {
        <::__pb::Mut<[u8]>>::from_inner(
          ::__pb::__internal::Private,
          ::__pb::__internal::RawVTableMutator::new(
            ::__pb::__internal::Private,
            ::__pb::__runtime::MutatorMessageRef::new(
              ::__pb::__internal::Private, &mut self.inner),
            &VTABLE,
          )
        )
      }
    }


  }  // impl Constant

  impl ::__std::ops::Drop for Constant {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_Operand_Constant_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_Operand_Constant_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_Operand_Constant_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_Operand_Constant_value(raw_msg: ::__pb::__internal::RawMessage) -> ::__pb::__internal::PtrAndLen;
    fn pilout_Operand_Constant_set_value(raw_msg: ::__pb::__internal::RawMessage, val: ::__pb::__internal::PtrAndLen);
    fn pilout_Operand_Constant_clear_value(raw_msg: ::__pb::__internal::RawMessage);


  }  // extern "C" for Constant

  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct Challenge {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `Challenge` does not provide shared mutation with its arena.
  // - `ChallengeMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for Challenge {}

  impl ::__pb::Proxied for Challenge {
    type View<'a> = ChallengeView<'a>;
    type Mut<'a> = ChallengeMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct ChallengeView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> ChallengeView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
    pub fn r#stage(&self) -> u32 { unsafe {
      pilout_Operand_Challenge_stage(self.msg)
    } }

    pub fn r#idx(&self) -> u32 { unsafe {
      pilout_Operand_Challenge_idx(self.msg)
    } }

  }

  // SAFETY:
  // - `ChallengeView` does not perform any mutation.
  // - While a `ChallengeView` exists, a `ChallengeMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `ChallengeMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for ChallengeView<'_> {}
  unsafe impl Send for ChallengeView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for ChallengeView<'a> {
    type Proxied = Challenge;

    fn as_view(&self) -> ::__pb::View<'a, Challenge> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Challenge> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<Challenge> for ChallengeView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<Challenge>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct ChallengeMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `ChallengeMut` does not perform any shared mutation.
  // - `ChallengeMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for ChallengeMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for ChallengeMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, Challenge> {
      ChallengeMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, Challenge> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for ChallengeMut<'a> {
    type Proxied = Challenge;
    fn as_view(&self) -> ::__pb::View<'_, Challenge> {
      ChallengeView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Challenge> where 'a: 'shorter {
      ChallengeView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl Challenge {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_Operand_Challenge_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_Operand_Challenge_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_Operand_Challenge_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // stage: optional uint32
    pub fn r#stage(&self) -> u32 {
      unsafe { pilout_Operand_Challenge_stage(self.inner.msg) }
    }
    pub fn r#stage_mut(&mut self) -> ::__pb::PrimitiveMut<'_, u32> {
      static VTABLE: ::__pb::__internal::PrimitiveVTable<u32> =
        ::__pb::__internal::PrimitiveVTable::new(
          ::__pb::__internal::Private,
          pilout_Operand_Challenge_stage,
          pilout_Operand_Challenge_set_stage,
        );

        ::__pb::PrimitiveMut::from_inner(
          ::__pb::__internal::Private,
          unsafe {
            ::__pb::__internal::RawVTableMutator::new(
              ::__pb::__internal::Private,
              ::__pb::__runtime::MutatorMessageRef::new(
                ::__pb::__internal::Private, &mut self.inner
              ),
              &VTABLE,
            )
          },
        )
    }

    // idx: optional uint32
    pub fn r#idx(&self) -> u32 {
      unsafe { pilout_Operand_Challenge_idx(self.inner.msg) }
    }
    pub fn r#idx_mut(&mut self) -> ::__pb::PrimitiveMut<'_, u32> {
      static VTABLE: ::__pb::__internal::PrimitiveVTable<u32> =
        ::__pb::__internal::PrimitiveVTable::new(
          ::__pb::__internal::Private,
          pilout_Operand_Challenge_idx,
          pilout_Operand_Challenge_set_idx,
        );

        ::__pb::PrimitiveMut::from_inner(
          ::__pb::__internal::Private,
          unsafe {
            ::__pb::__internal::RawVTableMutator::new(
              ::__pb::__internal::Private,
              ::__pb::__runtime::MutatorMessageRef::new(
                ::__pb::__internal::Private, &mut self.inner
              ),
              &VTABLE,
            )
          },
        )
    }


  }  // impl Challenge

  impl ::__std::ops::Drop for Challenge {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_Operand_Challenge_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_Operand_Challenge_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_Operand_Challenge_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_Operand_Challenge_stage(raw_msg: ::__pb::__internal::RawMessage) -> u32;
    fn pilout_Operand_Challenge_set_stage(raw_msg: ::__pb::__internal::RawMessage, val: u32);
    fn pilout_Operand_Challenge_clear_stage(raw_msg: ::__pb::__internal::RawMessage);

    fn pilout_Operand_Challenge_idx(raw_msg: ::__pb::__internal::RawMessage) -> u32;
    fn pilout_Operand_Challenge_set_idx(raw_msg: ::__pb::__internal::RawMessage, val: u32);
    fn pilout_Operand_Challenge_clear_idx(raw_msg: ::__pb::__internal::RawMessage);


  }  // extern "C" for Challenge

  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct ProofValue {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `ProofValue` does not provide shared mutation with its arena.
  // - `ProofValueMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for ProofValue {}

  impl ::__pb::Proxied for ProofValue {
    type View<'a> = ProofValueView<'a>;
    type Mut<'a> = ProofValueMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct ProofValueView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> ProofValueView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
    pub fn r#idx(&self) -> u32 { unsafe {
      pilout_Operand_ProofValue_idx(self.msg)
    } }

  }

  // SAFETY:
  // - `ProofValueView` does not perform any mutation.
  // - While a `ProofValueView` exists, a `ProofValueMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `ProofValueMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for ProofValueView<'_> {}
  unsafe impl Send for ProofValueView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for ProofValueView<'a> {
    type Proxied = ProofValue;

    fn as_view(&self) -> ::__pb::View<'a, ProofValue> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, ProofValue> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<ProofValue> for ProofValueView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<ProofValue>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct ProofValueMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `ProofValueMut` does not perform any shared mutation.
  // - `ProofValueMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for ProofValueMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for ProofValueMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, ProofValue> {
      ProofValueMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, ProofValue> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for ProofValueMut<'a> {
    type Proxied = ProofValue;
    fn as_view(&self) -> ::__pb::View<'_, ProofValue> {
      ProofValueView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, ProofValue> where 'a: 'shorter {
      ProofValueView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl ProofValue {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_Operand_ProofValue_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_Operand_ProofValue_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_Operand_ProofValue_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // idx: optional uint32
    pub fn r#idx(&self) -> u32 {
      unsafe { pilout_Operand_ProofValue_idx(self.inner.msg) }
    }
    pub fn r#idx_mut(&mut self) -> ::__pb::PrimitiveMut<'_, u32> {
      static VTABLE: ::__pb::__internal::PrimitiveVTable<u32> =
        ::__pb::__internal::PrimitiveVTable::new(
          ::__pb::__internal::Private,
          pilout_Operand_ProofValue_idx,
          pilout_Operand_ProofValue_set_idx,
        );

        ::__pb::PrimitiveMut::from_inner(
          ::__pb::__internal::Private,
          unsafe {
            ::__pb::__internal::RawVTableMutator::new(
              ::__pb::__internal::Private,
              ::__pb::__runtime::MutatorMessageRef::new(
                ::__pb::__internal::Private, &mut self.inner
              ),
              &VTABLE,
            )
          },
        )
    }


  }  // impl ProofValue

  impl ::__std::ops::Drop for ProofValue {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_Operand_ProofValue_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_Operand_ProofValue_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_Operand_ProofValue_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_Operand_ProofValue_idx(raw_msg: ::__pb::__internal::RawMessage) -> u32;
    fn pilout_Operand_ProofValue_set_idx(raw_msg: ::__pb::__internal::RawMessage, val: u32);
    fn pilout_Operand_ProofValue_clear_idx(raw_msg: ::__pb::__internal::RawMessage);


  }  // extern "C" for ProofValue

  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct SubproofValue {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `SubproofValue` does not provide shared mutation with its arena.
  // - `SubproofValueMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for SubproofValue {}

  impl ::__pb::Proxied for SubproofValue {
    type View<'a> = SubproofValueView<'a>;
    type Mut<'a> = SubproofValueMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct SubproofValueView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> SubproofValueView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
    pub fn r#idx(&self) -> u32 { unsafe {
      pilout_Operand_SubproofValue_idx(self.msg)
    } }

  }

  // SAFETY:
  // - `SubproofValueView` does not perform any mutation.
  // - While a `SubproofValueView` exists, a `SubproofValueMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `SubproofValueMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for SubproofValueView<'_> {}
  unsafe impl Send for SubproofValueView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for SubproofValueView<'a> {
    type Proxied = SubproofValue;

    fn as_view(&self) -> ::__pb::View<'a, SubproofValue> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, SubproofValue> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<SubproofValue> for SubproofValueView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<SubproofValue>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct SubproofValueMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `SubproofValueMut` does not perform any shared mutation.
  // - `SubproofValueMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for SubproofValueMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for SubproofValueMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, SubproofValue> {
      SubproofValueMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, SubproofValue> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for SubproofValueMut<'a> {
    type Proxied = SubproofValue;
    fn as_view(&self) -> ::__pb::View<'_, SubproofValue> {
      SubproofValueView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, SubproofValue> where 'a: 'shorter {
      SubproofValueView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl SubproofValue {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_Operand_SubproofValue_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_Operand_SubproofValue_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_Operand_SubproofValue_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // idx: optional uint32
    pub fn r#idx(&self) -> u32 {
      unsafe { pilout_Operand_SubproofValue_idx(self.inner.msg) }
    }
    pub fn r#idx_mut(&mut self) -> ::__pb::PrimitiveMut<'_, u32> {
      static VTABLE: ::__pb::__internal::PrimitiveVTable<u32> =
        ::__pb::__internal::PrimitiveVTable::new(
          ::__pb::__internal::Private,
          pilout_Operand_SubproofValue_idx,
          pilout_Operand_SubproofValue_set_idx,
        );

        ::__pb::PrimitiveMut::from_inner(
          ::__pb::__internal::Private,
          unsafe {
            ::__pb::__internal::RawVTableMutator::new(
              ::__pb::__internal::Private,
              ::__pb::__runtime::MutatorMessageRef::new(
                ::__pb::__internal::Private, &mut self.inner
              ),
              &VTABLE,
            )
          },
        )
    }


  }  // impl SubproofValue

  impl ::__std::ops::Drop for SubproofValue {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_Operand_SubproofValue_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_Operand_SubproofValue_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_Operand_SubproofValue_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_Operand_SubproofValue_idx(raw_msg: ::__pb::__internal::RawMessage) -> u32;
    fn pilout_Operand_SubproofValue_set_idx(raw_msg: ::__pb::__internal::RawMessage, val: u32);
    fn pilout_Operand_SubproofValue_clear_idx(raw_msg: ::__pb::__internal::RawMessage);


  }  // extern "C" for SubproofValue

  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct PublicValue {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `PublicValue` does not provide shared mutation with its arena.
  // - `PublicValueMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for PublicValue {}

  impl ::__pb::Proxied for PublicValue {
    type View<'a> = PublicValueView<'a>;
    type Mut<'a> = PublicValueMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct PublicValueView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> PublicValueView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
    pub fn r#idx(&self) -> u32 { unsafe {
      pilout_Operand_PublicValue_idx(self.msg)
    } }

  }

  // SAFETY:
  // - `PublicValueView` does not perform any mutation.
  // - While a `PublicValueView` exists, a `PublicValueMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `PublicValueMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for PublicValueView<'_> {}
  unsafe impl Send for PublicValueView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for PublicValueView<'a> {
    type Proxied = PublicValue;

    fn as_view(&self) -> ::__pb::View<'a, PublicValue> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, PublicValue> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<PublicValue> for PublicValueView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<PublicValue>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct PublicValueMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `PublicValueMut` does not perform any shared mutation.
  // - `PublicValueMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for PublicValueMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for PublicValueMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, PublicValue> {
      PublicValueMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, PublicValue> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for PublicValueMut<'a> {
    type Proxied = PublicValue;
    fn as_view(&self) -> ::__pb::View<'_, PublicValue> {
      PublicValueView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, PublicValue> where 'a: 'shorter {
      PublicValueView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl PublicValue {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_Operand_PublicValue_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_Operand_PublicValue_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_Operand_PublicValue_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // idx: optional uint32
    pub fn r#idx(&self) -> u32 {
      unsafe { pilout_Operand_PublicValue_idx(self.inner.msg) }
    }
    pub fn r#idx_mut(&mut self) -> ::__pb::PrimitiveMut<'_, u32> {
      static VTABLE: ::__pb::__internal::PrimitiveVTable<u32> =
        ::__pb::__internal::PrimitiveVTable::new(
          ::__pb::__internal::Private,
          pilout_Operand_PublicValue_idx,
          pilout_Operand_PublicValue_set_idx,
        );

        ::__pb::PrimitiveMut::from_inner(
          ::__pb::__internal::Private,
          unsafe {
            ::__pb::__internal::RawVTableMutator::new(
              ::__pb::__internal::Private,
              ::__pb::__runtime::MutatorMessageRef::new(
                ::__pb::__internal::Private, &mut self.inner
              ),
              &VTABLE,
            )
          },
        )
    }


  }  // impl PublicValue

  impl ::__std::ops::Drop for PublicValue {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_Operand_PublicValue_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_Operand_PublicValue_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_Operand_PublicValue_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_Operand_PublicValue_idx(raw_msg: ::__pb::__internal::RawMessage) -> u32;
    fn pilout_Operand_PublicValue_set_idx(raw_msg: ::__pb::__internal::RawMessage, val: u32);
    fn pilout_Operand_PublicValue_clear_idx(raw_msg: ::__pb::__internal::RawMessage);


  }  // extern "C" for PublicValue

  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct PeriodicCol {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `PeriodicCol` does not provide shared mutation with its arena.
  // - `PeriodicColMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for PeriodicCol {}

  impl ::__pb::Proxied for PeriodicCol {
    type View<'a> = PeriodicColView<'a>;
    type Mut<'a> = PeriodicColMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct PeriodicColView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> PeriodicColView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
    pub fn r#idx(&self) -> u32 { unsafe {
      pilout_Operand_PeriodicCol_idx(self.msg)
    } }

    pub fn r#rowOffset(&self) -> i32 { unsafe {
      pilout_Operand_PeriodicCol_rowOffset(self.msg)
    } }

  }

  // SAFETY:
  // - `PeriodicColView` does not perform any mutation.
  // - While a `PeriodicColView` exists, a `PeriodicColMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `PeriodicColMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for PeriodicColView<'_> {}
  unsafe impl Send for PeriodicColView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for PeriodicColView<'a> {
    type Proxied = PeriodicCol;

    fn as_view(&self) -> ::__pb::View<'a, PeriodicCol> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, PeriodicCol> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<PeriodicCol> for PeriodicColView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<PeriodicCol>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct PeriodicColMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `PeriodicColMut` does not perform any shared mutation.
  // - `PeriodicColMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for PeriodicColMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for PeriodicColMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, PeriodicCol> {
      PeriodicColMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, PeriodicCol> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for PeriodicColMut<'a> {
    type Proxied = PeriodicCol;
    fn as_view(&self) -> ::__pb::View<'_, PeriodicCol> {
      PeriodicColView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, PeriodicCol> where 'a: 'shorter {
      PeriodicColView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl PeriodicCol {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_Operand_PeriodicCol_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_Operand_PeriodicCol_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_Operand_PeriodicCol_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // idx: optional uint32
    pub fn r#idx(&self) -> u32 {
      unsafe { pilout_Operand_PeriodicCol_idx(self.inner.msg) }
    }
    pub fn r#idx_mut(&mut self) -> ::__pb::PrimitiveMut<'_, u32> {
      static VTABLE: ::__pb::__internal::PrimitiveVTable<u32> =
        ::__pb::__internal::PrimitiveVTable::new(
          ::__pb::__internal::Private,
          pilout_Operand_PeriodicCol_idx,
          pilout_Operand_PeriodicCol_set_idx,
        );

        ::__pb::PrimitiveMut::from_inner(
          ::__pb::__internal::Private,
          unsafe {
            ::__pb::__internal::RawVTableMutator::new(
              ::__pb::__internal::Private,
              ::__pb::__runtime::MutatorMessageRef::new(
                ::__pb::__internal::Private, &mut self.inner
              ),
              &VTABLE,
            )
          },
        )
    }

    // rowOffset: optional sint32
    pub fn r#rowOffset(&self) -> i32 {
      unsafe { pilout_Operand_PeriodicCol_rowOffset(self.inner.msg) }
    }
    pub fn r#rowOffset_mut(&mut self) -> ::__pb::PrimitiveMut<'_, i32> {
      static VTABLE: ::__pb::__internal::PrimitiveVTable<i32> =
        ::__pb::__internal::PrimitiveVTable::new(
          ::__pb::__internal::Private,
          pilout_Operand_PeriodicCol_rowOffset,
          pilout_Operand_PeriodicCol_set_rowOffset,
        );

        ::__pb::PrimitiveMut::from_inner(
          ::__pb::__internal::Private,
          unsafe {
            ::__pb::__internal::RawVTableMutator::new(
              ::__pb::__internal::Private,
              ::__pb::__runtime::MutatorMessageRef::new(
                ::__pb::__internal::Private, &mut self.inner
              ),
              &VTABLE,
            )
          },
        )
    }


  }  // impl PeriodicCol

  impl ::__std::ops::Drop for PeriodicCol {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_Operand_PeriodicCol_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_Operand_PeriodicCol_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_Operand_PeriodicCol_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_Operand_PeriodicCol_idx(raw_msg: ::__pb::__internal::RawMessage) -> u32;
    fn pilout_Operand_PeriodicCol_set_idx(raw_msg: ::__pb::__internal::RawMessage, val: u32);
    fn pilout_Operand_PeriodicCol_clear_idx(raw_msg: ::__pb::__internal::RawMessage);

    fn pilout_Operand_PeriodicCol_rowOffset(raw_msg: ::__pb::__internal::RawMessage) -> i32;
    fn pilout_Operand_PeriodicCol_set_rowOffset(raw_msg: ::__pb::__internal::RawMessage, val: i32);
    fn pilout_Operand_PeriodicCol_clear_rowOffset(raw_msg: ::__pb::__internal::RawMessage);


  }  // extern "C" for PeriodicCol

  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct FixedCol {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `FixedCol` does not provide shared mutation with its arena.
  // - `FixedColMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for FixedCol {}

  impl ::__pb::Proxied for FixedCol {
    type View<'a> = FixedColView<'a>;
    type Mut<'a> = FixedColMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct FixedColView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> FixedColView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
    pub fn r#idx(&self) -> u32 { unsafe {
      pilout_Operand_FixedCol_idx(self.msg)
    } }

    pub fn r#rowOffset(&self) -> i32 { unsafe {
      pilout_Operand_FixedCol_rowOffset(self.msg)
    } }

  }

  // SAFETY:
  // - `FixedColView` does not perform any mutation.
  // - While a `FixedColView` exists, a `FixedColMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `FixedColMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for FixedColView<'_> {}
  unsafe impl Send for FixedColView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for FixedColView<'a> {
    type Proxied = FixedCol;

    fn as_view(&self) -> ::__pb::View<'a, FixedCol> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, FixedCol> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<FixedCol> for FixedColView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<FixedCol>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct FixedColMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `FixedColMut` does not perform any shared mutation.
  // - `FixedColMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for FixedColMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for FixedColMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, FixedCol> {
      FixedColMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, FixedCol> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for FixedColMut<'a> {
    type Proxied = FixedCol;
    fn as_view(&self) -> ::__pb::View<'_, FixedCol> {
      FixedColView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, FixedCol> where 'a: 'shorter {
      FixedColView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl FixedCol {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_Operand_FixedCol_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_Operand_FixedCol_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_Operand_FixedCol_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // idx: optional uint32
    pub fn r#idx(&self) -> u32 {
      unsafe { pilout_Operand_FixedCol_idx(self.inner.msg) }
    }
    pub fn r#idx_mut(&mut self) -> ::__pb::PrimitiveMut<'_, u32> {
      static VTABLE: ::__pb::__internal::PrimitiveVTable<u32> =
        ::__pb::__internal::PrimitiveVTable::new(
          ::__pb::__internal::Private,
          pilout_Operand_FixedCol_idx,
          pilout_Operand_FixedCol_set_idx,
        );

        ::__pb::PrimitiveMut::from_inner(
          ::__pb::__internal::Private,
          unsafe {
            ::__pb::__internal::RawVTableMutator::new(
              ::__pb::__internal::Private,
              ::__pb::__runtime::MutatorMessageRef::new(
                ::__pb::__internal::Private, &mut self.inner
              ),
              &VTABLE,
            )
          },
        )
    }

    // rowOffset: optional sint32
    pub fn r#rowOffset(&self) -> i32 {
      unsafe { pilout_Operand_FixedCol_rowOffset(self.inner.msg) }
    }
    pub fn r#rowOffset_mut(&mut self) -> ::__pb::PrimitiveMut<'_, i32> {
      static VTABLE: ::__pb::__internal::PrimitiveVTable<i32> =
        ::__pb::__internal::PrimitiveVTable::new(
          ::__pb::__internal::Private,
          pilout_Operand_FixedCol_rowOffset,
          pilout_Operand_FixedCol_set_rowOffset,
        );

        ::__pb::PrimitiveMut::from_inner(
          ::__pb::__internal::Private,
          unsafe {
            ::__pb::__internal::RawVTableMutator::new(
              ::__pb::__internal::Private,
              ::__pb::__runtime::MutatorMessageRef::new(
                ::__pb::__internal::Private, &mut self.inner
              ),
              &VTABLE,
            )
          },
        )
    }


  }  // impl FixedCol

  impl ::__std::ops::Drop for FixedCol {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_Operand_FixedCol_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_Operand_FixedCol_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_Operand_FixedCol_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_Operand_FixedCol_idx(raw_msg: ::__pb::__internal::RawMessage) -> u32;
    fn pilout_Operand_FixedCol_set_idx(raw_msg: ::__pb::__internal::RawMessage, val: u32);
    fn pilout_Operand_FixedCol_clear_idx(raw_msg: ::__pb::__internal::RawMessage);

    fn pilout_Operand_FixedCol_rowOffset(raw_msg: ::__pb::__internal::RawMessage) -> i32;
    fn pilout_Operand_FixedCol_set_rowOffset(raw_msg: ::__pb::__internal::RawMessage, val: i32);
    fn pilout_Operand_FixedCol_clear_rowOffset(raw_msg: ::__pb::__internal::RawMessage);


  }  // extern "C" for FixedCol

  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct WitnessCol {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `WitnessCol` does not provide shared mutation with its arena.
  // - `WitnessColMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for WitnessCol {}

  impl ::__pb::Proxied for WitnessCol {
    type View<'a> = WitnessColView<'a>;
    type Mut<'a> = WitnessColMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct WitnessColView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> WitnessColView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
    pub fn r#stage(&self) -> u32 { unsafe {
      pilout_Operand_WitnessCol_stage(self.msg)
    } }

    pub fn r#colIdx(&self) -> u32 { unsafe {
      pilout_Operand_WitnessCol_colIdx(self.msg)
    } }

    pub fn r#rowOffset(&self) -> i32 { unsafe {
      pilout_Operand_WitnessCol_rowOffset(self.msg)
    } }

  }

  // SAFETY:
  // - `WitnessColView` does not perform any mutation.
  // - While a `WitnessColView` exists, a `WitnessColMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `WitnessColMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for WitnessColView<'_> {}
  unsafe impl Send for WitnessColView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for WitnessColView<'a> {
    type Proxied = WitnessCol;

    fn as_view(&self) -> ::__pb::View<'a, WitnessCol> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, WitnessCol> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<WitnessCol> for WitnessColView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<WitnessCol>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct WitnessColMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `WitnessColMut` does not perform any shared mutation.
  // - `WitnessColMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for WitnessColMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for WitnessColMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, WitnessCol> {
      WitnessColMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, WitnessCol> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for WitnessColMut<'a> {
    type Proxied = WitnessCol;
    fn as_view(&self) -> ::__pb::View<'_, WitnessCol> {
      WitnessColView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, WitnessCol> where 'a: 'shorter {
      WitnessColView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl WitnessCol {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_Operand_WitnessCol_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_Operand_WitnessCol_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_Operand_WitnessCol_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // stage: optional uint32
    pub fn r#stage(&self) -> u32 {
      unsafe { pilout_Operand_WitnessCol_stage(self.inner.msg) }
    }
    pub fn r#stage_mut(&mut self) -> ::__pb::PrimitiveMut<'_, u32> {
      static VTABLE: ::__pb::__internal::PrimitiveVTable<u32> =
        ::__pb::__internal::PrimitiveVTable::new(
          ::__pb::__internal::Private,
          pilout_Operand_WitnessCol_stage,
          pilout_Operand_WitnessCol_set_stage,
        );

        ::__pb::PrimitiveMut::from_inner(
          ::__pb::__internal::Private,
          unsafe {
            ::__pb::__internal::RawVTableMutator::new(
              ::__pb::__internal::Private,
              ::__pb::__runtime::MutatorMessageRef::new(
                ::__pb::__internal::Private, &mut self.inner
              ),
              &VTABLE,
            )
          },
        )
    }

    // colIdx: optional uint32
    pub fn r#colIdx(&self) -> u32 {
      unsafe { pilout_Operand_WitnessCol_colIdx(self.inner.msg) }
    }
    pub fn r#colIdx_mut(&mut self) -> ::__pb::PrimitiveMut<'_, u32> {
      static VTABLE: ::__pb::__internal::PrimitiveVTable<u32> =
        ::__pb::__internal::PrimitiveVTable::new(
          ::__pb::__internal::Private,
          pilout_Operand_WitnessCol_colIdx,
          pilout_Operand_WitnessCol_set_colIdx,
        );

        ::__pb::PrimitiveMut::from_inner(
          ::__pb::__internal::Private,
          unsafe {
            ::__pb::__internal::RawVTableMutator::new(
              ::__pb::__internal::Private,
              ::__pb::__runtime::MutatorMessageRef::new(
                ::__pb::__internal::Private, &mut self.inner
              ),
              &VTABLE,
            )
          },
        )
    }

    // rowOffset: optional sint32
    pub fn r#rowOffset(&self) -> i32 {
      unsafe { pilout_Operand_WitnessCol_rowOffset(self.inner.msg) }
    }
    pub fn r#rowOffset_mut(&mut self) -> ::__pb::PrimitiveMut<'_, i32> {
      static VTABLE: ::__pb::__internal::PrimitiveVTable<i32> =
        ::__pb::__internal::PrimitiveVTable::new(
          ::__pb::__internal::Private,
          pilout_Operand_WitnessCol_rowOffset,
          pilout_Operand_WitnessCol_set_rowOffset,
        );

        ::__pb::PrimitiveMut::from_inner(
          ::__pb::__internal::Private,
          unsafe {
            ::__pb::__internal::RawVTableMutator::new(
              ::__pb::__internal::Private,
              ::__pb::__runtime::MutatorMessageRef::new(
                ::__pb::__internal::Private, &mut self.inner
              ),
              &VTABLE,
            )
          },
        )
    }


  }  // impl WitnessCol

  impl ::__std::ops::Drop for WitnessCol {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_Operand_WitnessCol_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_Operand_WitnessCol_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_Operand_WitnessCol_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_Operand_WitnessCol_stage(raw_msg: ::__pb::__internal::RawMessage) -> u32;
    fn pilout_Operand_WitnessCol_set_stage(raw_msg: ::__pb::__internal::RawMessage, val: u32);
    fn pilout_Operand_WitnessCol_clear_stage(raw_msg: ::__pb::__internal::RawMessage);

    fn pilout_Operand_WitnessCol_colIdx(raw_msg: ::__pb::__internal::RawMessage) -> u32;
    fn pilout_Operand_WitnessCol_set_colIdx(raw_msg: ::__pb::__internal::RawMessage, val: u32);
    fn pilout_Operand_WitnessCol_clear_colIdx(raw_msg: ::__pb::__internal::RawMessage);

    fn pilout_Operand_WitnessCol_rowOffset(raw_msg: ::__pb::__internal::RawMessage) -> i32;
    fn pilout_Operand_WitnessCol_set_rowOffset(raw_msg: ::__pb::__internal::RawMessage, val: i32);
    fn pilout_Operand_WitnessCol_clear_rowOffset(raw_msg: ::__pb::__internal::RawMessage);


  }  // extern "C" for WitnessCol

  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct Expression {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `Expression` does not provide shared mutation with its arena.
  // - `ExpressionMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for Expression {}

  impl ::__pb::Proxied for Expression {
    type View<'a> = ExpressionView<'a>;
    type Mut<'a> = ExpressionMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct ExpressionView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> ExpressionView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
    pub fn r#idx(&self) -> u32 { unsafe {
      pilout_Operand_Expression_idx(self.msg)
    } }

  }

  // SAFETY:
  // - `ExpressionView` does not perform any mutation.
  // - While a `ExpressionView` exists, a `ExpressionMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `ExpressionMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for ExpressionView<'_> {}
  unsafe impl Send for ExpressionView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for ExpressionView<'a> {
    type Proxied = Expression;

    fn as_view(&self) -> ::__pb::View<'a, Expression> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Expression> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<Expression> for ExpressionView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<Expression>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct ExpressionMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `ExpressionMut` does not perform any shared mutation.
  // - `ExpressionMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for ExpressionMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for ExpressionMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, Expression> {
      ExpressionMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, Expression> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for ExpressionMut<'a> {
    type Proxied = Expression;
    fn as_view(&self) -> ::__pb::View<'_, Expression> {
      ExpressionView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Expression> where 'a: 'shorter {
      ExpressionView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl Expression {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_Operand_Expression_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_Operand_Expression_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_Operand_Expression_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // idx: optional uint32
    pub fn r#idx(&self) -> u32 {
      unsafe { pilout_Operand_Expression_idx(self.inner.msg) }
    }
    pub fn r#idx_mut(&mut self) -> ::__pb::PrimitiveMut<'_, u32> {
      static VTABLE: ::__pb::__internal::PrimitiveVTable<u32> =
        ::__pb::__internal::PrimitiveVTable::new(
          ::__pb::__internal::Private,
          pilout_Operand_Expression_idx,
          pilout_Operand_Expression_set_idx,
        );

        ::__pb::PrimitiveMut::from_inner(
          ::__pb::__internal::Private,
          unsafe {
            ::__pb::__internal::RawVTableMutator::new(
              ::__pb::__internal::Private,
              ::__pb::__runtime::MutatorMessageRef::new(
                ::__pb::__internal::Private, &mut self.inner
              ),
              &VTABLE,
            )
          },
        )
    }


  }  // impl Expression

  impl ::__std::ops::Drop for Expression {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_Operand_Expression_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_Operand_Expression_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_Operand_Expression_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_Operand_Expression_idx(raw_msg: ::__pb::__internal::RawMessage) -> u32;
    fn pilout_Operand_Expression_set_idx(raw_msg: ::__pb::__internal::RawMessage, val: u32);
    fn pilout_Operand_Expression_clear_idx(raw_msg: ::__pb::__internal::RawMessage);


  }  // extern "C" for Expression


  #[non_exhaustive]
  #[derive(Debug)]
  #[allow(dead_code)]
  #[repr(isize)]
  pub enum Operand<'msg> {

    #[allow(non_camel_case_types)]
    not_set(std::marker::PhantomData<&'msg ()>) = 0
  }

  #[non_exhaustive]
  #[derive(Debug)]
  #[allow(dead_code)]
  #[repr(isize)]
  pub enum OperandMut<'msg> {

    #[allow(non_camel_case_types)]
    not_set(std::marker::PhantomData<&'msg ()>) = 0
  }
  #[repr(C)]
  #[derive(Debug, Copy, Clone, PartialEq, Eq)]
  pub(super) enum OperandCase {
    Constant = 1,
    Challenge = 2,
    ProofValue = 3,
    SubproofValue = 4,
    PublicValue = 5,
    PeriodicCol = 6,
    FixedCol = 7,
    WitnessCol = 8,
    Expression = 9,

    #[allow(non_camel_case_types)]
    not_set = 0
  }
}  // mod Operand_

#[allow(non_camel_case_types)]
// TODO: Implement support for debug redaction
#[derive(Debug)]
pub struct Expression {
  inner: ::__pb::__runtime::MessageInner
}

// SAFETY:
// - `Expression` does not provide shared mutation with its arena.
// - `ExpressionMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena that would conflict with
//   field access is impossible.
unsafe impl Sync for Expression {}

impl ::__pb::Proxied for Expression {
  type View<'a> = ExpressionView<'a>;
  type Mut<'a> = ExpressionMut<'a>;
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub struct ExpressionView<'a> {
  msg: ::__pb::__internal::RawMessage,
  _phantom: ::__std::marker::PhantomData<&'a ()>,
}

impl<'a> ExpressionView<'a> {
  #[doc(hidden)]
  pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
    Self { msg, _phantom: std::marker::PhantomData }
  }
}

// SAFETY:
// - `ExpressionView` does not perform any mutation.
// - While a `ExpressionView` exists, a `ExpressionMut` can't exist to mutate
//   the arena that would conflict with field access.
// - `ExpressionMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for ExpressionView<'_> {}
unsafe impl Send for ExpressionView<'_> {}

impl<'a> ::__pb::ViewProxy<'a> for ExpressionView<'a> {
  type Proxied = Expression;

  fn as_view(&self) -> ::__pb::View<'a, Expression> {
    *self
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Expression> where 'a: 'shorter {
    self
  }
}

impl<'a> ::__pb::SettableValue<Expression> for ExpressionView<'a> {
  fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<Expression>) {
    todo!()
  }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct ExpressionMut<'a> {
  inner: ::__pb::__runtime::MutatorMessageRef<'a>,
}

// SAFETY:
// - `ExpressionMut` does not perform any shared mutation.
// - `ExpressionMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for ExpressionMut<'_> {}

impl<'a> ::__pb::MutProxy<'a> for ExpressionMut<'a> {
  fn as_mut(&mut self) -> ::__pb::Mut<'_, Expression> {
    ExpressionMut { inner: self.inner }
  }
  fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, Expression> where 'a : 'shorter { self }
}

impl<'a> ::__pb::ViewProxy<'a> for ExpressionMut<'a> {
  type Proxied = Expression;
  fn as_view(&self) -> ::__pb::View<'_, Expression> {
    ExpressionView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Expression> where 'a: 'shorter {
    ExpressionView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
}

impl Expression {
  pub fn new() -> Self {
    let arena = ::__pb::__runtime::Arena::new();
    Self {
      inner: ::__pb::__runtime::MessageInner {
        msg: unsafe { pilout_Expression_new(arena.raw()) },
        arena,
      }
    }
  }

  pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
    let arena = ::__pb::__runtime::Arena::new();
    let mut len = 0;
    unsafe {
      let data = pilout_Expression_serialize(self.inner.msg, arena.raw(), &mut len);
      ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
    }
  }
  pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
    let arena = ::__pb::__runtime::Arena::new();
    let msg = unsafe {
      pilout_Expression_parse(data.as_ptr(), data.len(), arena.raw())
    };

    match msg {
      None => Err(::__pb::ParseError),
      Some(msg) => {
        // This assignment causes self.arena to be dropped and to deallocate
        // any previous message pointed/owned to by self.inner.msg.
        self.inner.arena = arena;
        self.inner.msg = msg;
        Ok(())
      }
    }
  }

  // add: optional message pilout.Expression.Add
  pub fn r#add(&self) -> crate::pilout::Expression_::AddView {
    let submsg = unsafe { pilout_Expression_add(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::Expression_::AddView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::Expression_::AddView::new(::__pb::__internal::Private, field),
      }
  }

  // sub: optional message pilout.Expression.Sub
  pub fn r#sub(&self) -> crate::pilout::Expression_::SubView {
    let submsg = unsafe { pilout_Expression_sub(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::Expression_::SubView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::Expression_::SubView::new(::__pb::__internal::Private, field),
      }
  }

  // mul: optional message pilout.Expression.Mul
  pub fn r#mul(&self) -> crate::pilout::Expression_::MulView {
    let submsg = unsafe { pilout_Expression_mul(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::Expression_::MulView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::Expression_::MulView::new(::__pb::__internal::Private, field),
      }
  }

  // neg: optional message pilout.Expression.Neg
  pub fn r#neg(&self) -> crate::pilout::Expression_::NegView {
    let submsg = unsafe { pilout_Expression_neg(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::Expression_::NegView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::Expression_::NegView::new(::__pb::__internal::Private, field),
      }
  }


  pub fn r#operation(&self) -> Expression_::Operation {
    match unsafe { pilout_Expression_operation_case(self.inner.msg) } {
      _ => Expression_::Operation::not_set(std::marker::PhantomData)
    }
  }

  pub fn r#operation_mut(&mut self) -> Expression_::OperationMut {
    match unsafe { pilout_Expression_operation_case(self.inner.msg) } {
      _ => Expression_::OperationMut::not_set(std::marker::PhantomData)
    }
  }

}  // impl Expression

impl ::__std::ops::Drop for Expression {
  fn drop(&mut self) {
  }
}

extern "C" {
  fn pilout_Expression_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
  fn pilout_Expression_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
  fn pilout_Expression_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_Expression_add(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_Expression_sub(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_Expression_mul(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_Expression_neg(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;


  fn pilout_Expression_operation_case(raw_msg: ::__pb::__internal::RawMessage) -> Expression_::OperationCase;

}  // extern "C" for Expression

#[allow(non_snake_case)]
pub mod Expression_ {
  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct Add {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `Add` does not provide shared mutation with its arena.
  // - `AddMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for Add {}

  impl ::__pb::Proxied for Add {
    type View<'a> = AddView<'a>;
    type Mut<'a> = AddMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct AddView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> AddView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
  }

  // SAFETY:
  // - `AddView` does not perform any mutation.
  // - While a `AddView` exists, a `AddMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `AddMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for AddView<'_> {}
  unsafe impl Send for AddView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for AddView<'a> {
    type Proxied = Add;

    fn as_view(&self) -> ::__pb::View<'a, Add> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Add> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<Add> for AddView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<Add>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct AddMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `AddMut` does not perform any shared mutation.
  // - `AddMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for AddMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for AddMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, Add> {
      AddMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, Add> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for AddMut<'a> {
    type Proxied = Add;
    fn as_view(&self) -> ::__pb::View<'_, Add> {
      AddView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Add> where 'a: 'shorter {
      AddView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl Add {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_Expression_Add_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_Expression_Add_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_Expression_Add_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // lhs: optional message pilout.Operand
    pub fn r#lhs(&self) -> crate::pilout::OperandView {
      let submsg = unsafe { pilout_Expression_Add_lhs(self.inner.msg) };
      // For upb, getters return null if the field is unset, so we need to
      // check for null and return the default instance manually. Note that
      // a null ptr received from upb manifests as Option::None
      match submsg {
          // TODO:(b/304357029)
          None => crate::pilout::OperandView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
          Some(field) => crate::pilout::OperandView::new(::__pb::__internal::Private, field),
        }
    }

    // rhs: optional message pilout.Operand
    pub fn r#rhs(&self) -> crate::pilout::OperandView {
      let submsg = unsafe { pilout_Expression_Add_rhs(self.inner.msg) };
      // For upb, getters return null if the field is unset, so we need to
      // check for null and return the default instance manually. Note that
      // a null ptr received from upb manifests as Option::None
      match submsg {
          // TODO:(b/304357029)
          None => crate::pilout::OperandView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
          Some(field) => crate::pilout::OperandView::new(::__pb::__internal::Private, field),
        }
    }


  }  // impl Add

  impl ::__std::ops::Drop for Add {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_Expression_Add_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_Expression_Add_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_Expression_Add_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_Expression_Add_lhs(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_Expression_Add_rhs(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;


  }  // extern "C" for Add

  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct Sub {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `Sub` does not provide shared mutation with its arena.
  // - `SubMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for Sub {}

  impl ::__pb::Proxied for Sub {
    type View<'a> = SubView<'a>;
    type Mut<'a> = SubMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct SubView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> SubView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
  }

  // SAFETY:
  // - `SubView` does not perform any mutation.
  // - While a `SubView` exists, a `SubMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `SubMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for SubView<'_> {}
  unsafe impl Send for SubView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for SubView<'a> {
    type Proxied = Sub;

    fn as_view(&self) -> ::__pb::View<'a, Sub> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Sub> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<Sub> for SubView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<Sub>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct SubMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `SubMut` does not perform any shared mutation.
  // - `SubMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for SubMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for SubMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, Sub> {
      SubMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, Sub> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for SubMut<'a> {
    type Proxied = Sub;
    fn as_view(&self) -> ::__pb::View<'_, Sub> {
      SubView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Sub> where 'a: 'shorter {
      SubView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl Sub {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_Expression_Sub_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_Expression_Sub_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_Expression_Sub_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // lhs: optional message pilout.Operand
    pub fn r#lhs(&self) -> crate::pilout::OperandView {
      let submsg = unsafe { pilout_Expression_Sub_lhs(self.inner.msg) };
      // For upb, getters return null if the field is unset, so we need to
      // check for null and return the default instance manually. Note that
      // a null ptr received from upb manifests as Option::None
      match submsg {
          // TODO:(b/304357029)
          None => crate::pilout::OperandView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
          Some(field) => crate::pilout::OperandView::new(::__pb::__internal::Private, field),
        }
    }

    // rhs: optional message pilout.Operand
    pub fn r#rhs(&self) -> crate::pilout::OperandView {
      let submsg = unsafe { pilout_Expression_Sub_rhs(self.inner.msg) };
      // For upb, getters return null if the field is unset, so we need to
      // check for null and return the default instance manually. Note that
      // a null ptr received from upb manifests as Option::None
      match submsg {
          // TODO:(b/304357029)
          None => crate::pilout::OperandView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
          Some(field) => crate::pilout::OperandView::new(::__pb::__internal::Private, field),
        }
    }


  }  // impl Sub

  impl ::__std::ops::Drop for Sub {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_Expression_Sub_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_Expression_Sub_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_Expression_Sub_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_Expression_Sub_lhs(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_Expression_Sub_rhs(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;


  }  // extern "C" for Sub

  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct Mul {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `Mul` does not provide shared mutation with its arena.
  // - `MulMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for Mul {}

  impl ::__pb::Proxied for Mul {
    type View<'a> = MulView<'a>;
    type Mut<'a> = MulMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct MulView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> MulView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
  }

  // SAFETY:
  // - `MulView` does not perform any mutation.
  // - While a `MulView` exists, a `MulMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `MulMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for MulView<'_> {}
  unsafe impl Send for MulView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for MulView<'a> {
    type Proxied = Mul;

    fn as_view(&self) -> ::__pb::View<'a, Mul> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Mul> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<Mul> for MulView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<Mul>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct MulMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `MulMut` does not perform any shared mutation.
  // - `MulMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for MulMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for MulMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, Mul> {
      MulMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, Mul> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for MulMut<'a> {
    type Proxied = Mul;
    fn as_view(&self) -> ::__pb::View<'_, Mul> {
      MulView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Mul> where 'a: 'shorter {
      MulView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl Mul {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_Expression_Mul_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_Expression_Mul_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_Expression_Mul_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // lhs: optional message pilout.Operand
    pub fn r#lhs(&self) -> crate::pilout::OperandView {
      let submsg = unsafe { pilout_Expression_Mul_lhs(self.inner.msg) };
      // For upb, getters return null if the field is unset, so we need to
      // check for null and return the default instance manually. Note that
      // a null ptr received from upb manifests as Option::None
      match submsg {
          // TODO:(b/304357029)
          None => crate::pilout::OperandView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
          Some(field) => crate::pilout::OperandView::new(::__pb::__internal::Private, field),
        }
    }

    // rhs: optional message pilout.Operand
    pub fn r#rhs(&self) -> crate::pilout::OperandView {
      let submsg = unsafe { pilout_Expression_Mul_rhs(self.inner.msg) };
      // For upb, getters return null if the field is unset, so we need to
      // check for null and return the default instance manually. Note that
      // a null ptr received from upb manifests as Option::None
      match submsg {
          // TODO:(b/304357029)
          None => crate::pilout::OperandView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
          Some(field) => crate::pilout::OperandView::new(::__pb::__internal::Private, field),
        }
    }


  }  // impl Mul

  impl ::__std::ops::Drop for Mul {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_Expression_Mul_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_Expression_Mul_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_Expression_Mul_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_Expression_Mul_lhs(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_Expression_Mul_rhs(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;


  }  // extern "C" for Mul

  #[allow(non_camel_case_types)]
  // TODO: Implement support for debug redaction
  #[derive(Debug)]
  pub struct Neg {
    inner: ::__pb::__runtime::MessageInner
  }

  // SAFETY:
  // - `Neg` does not provide shared mutation with its arena.
  // - `NegMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena that would conflict with
  //   field access is impossible.
  unsafe impl Sync for Neg {}

  impl ::__pb::Proxied for Neg {
    type View<'a> = NegView<'a>;
    type Mut<'a> = NegMut<'a>;
  }

  #[derive(Debug, Copy, Clone)]
  #[allow(dead_code)]
  pub struct NegView<'a> {
    msg: ::__pb::__internal::RawMessage,
    _phantom: ::__std::marker::PhantomData<&'a ()>,
  }

  impl<'a> NegView<'a> {
    #[doc(hidden)]
    pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
      Self { msg, _phantom: std::marker::PhantomData }
    }
  }

  // SAFETY:
  // - `NegView` does not perform any mutation.
  // - While a `NegView` exists, a `NegMut` can't exist to mutate
  //   the arena that would conflict with field access.
  // - `NegMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for NegView<'_> {}
  unsafe impl Send for NegView<'_> {}

  impl<'a> ::__pb::ViewProxy<'a> for NegView<'a> {
    type Proxied = Neg;

    fn as_view(&self) -> ::__pb::View<'a, Neg> {
      *self
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Neg> where 'a: 'shorter {
      self
    }
  }

  impl<'a> ::__pb::SettableValue<Neg> for NegView<'a> {
    fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<Neg>) {
      todo!()
    }
  }

  #[derive(Debug)]
  #[allow(dead_code)]
  pub struct NegMut<'a> {
    inner: ::__pb::__runtime::MutatorMessageRef<'a>,
  }

  // SAFETY:
  // - `NegMut` does not perform any shared mutation.
  // - `NegMut` is not `Send`, and so even in the presence of mutator
  //   splitting, synchronous access of an arena is impossible.
  unsafe impl Sync for NegMut<'_> {}

  impl<'a> ::__pb::MutProxy<'a> for NegMut<'a> {
    fn as_mut(&mut self) -> ::__pb::Mut<'_, Neg> {
      NegMut { inner: self.inner }
    }
    fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, Neg> where 'a : 'shorter { self }
  }

  impl<'a> ::__pb::ViewProxy<'a> for NegMut<'a> {
    type Proxied = Neg;
    fn as_view(&self) -> ::__pb::View<'_, Neg> {
      NegView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
    fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Neg> where 'a: 'shorter {
      NegView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
    }
  }

  impl Neg {
    pub fn new() -> Self {
      let arena = ::__pb::__runtime::Arena::new();
      Self {
        inner: ::__pb::__runtime::MessageInner {
          msg: unsafe { pilout_Expression_Neg_new(arena.raw()) },
          arena,
        }
      }
    }

    pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
      let arena = ::__pb::__runtime::Arena::new();
      let mut len = 0;
      unsafe {
        let data = pilout_Expression_Neg_serialize(self.inner.msg, arena.raw(), &mut len);
        ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
      }
    }
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
      let arena = ::__pb::__runtime::Arena::new();
      let msg = unsafe {
        pilout_Expression_Neg_parse(data.as_ptr(), data.len(), arena.raw())
      };

      match msg {
        None => Err(::__pb::ParseError),
        Some(msg) => {
          // This assignment causes self.arena to be dropped and to deallocate
          // any previous message pointed/owned to by self.inner.msg.
          self.inner.arena = arena;
          self.inner.msg = msg;
          Ok(())
        }
      }
    }

    // value: optional message pilout.Operand
    pub fn r#value(&self) -> crate::pilout::OperandView {
      let submsg = unsafe { pilout_Expression_Neg_value(self.inner.msg) };
      // For upb, getters return null if the field is unset, so we need to
      // check for null and return the default instance manually. Note that
      // a null ptr received from upb manifests as Option::None
      match submsg {
          // TODO:(b/304357029)
          None => crate::pilout::OperandView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
          Some(field) => crate::pilout::OperandView::new(::__pb::__internal::Private, field),
        }
    }


  }  // impl Neg

  impl ::__std::ops::Drop for Neg {
    fn drop(&mut self) {
    }
  }

  extern "C" {
    fn pilout_Expression_Neg_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
    fn pilout_Expression_Neg_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
    fn pilout_Expression_Neg_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

    fn pilout_Expression_Neg_value(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;


  }  // extern "C" for Neg


  #[non_exhaustive]
  #[derive(Debug)]
  #[allow(dead_code)]
  #[repr(isize)]
  pub enum Operation<'msg> {

    #[allow(non_camel_case_types)]
    not_set(std::marker::PhantomData<&'msg ()>) = 0
  }

  #[non_exhaustive]
  #[derive(Debug)]
  #[allow(dead_code)]
  #[repr(isize)]
  pub enum OperationMut<'msg> {

    #[allow(non_camel_case_types)]
    not_set(std::marker::PhantomData<&'msg ()>) = 0
  }
  #[repr(C)]
  #[derive(Debug, Copy, Clone, PartialEq, Eq)]
  pub(super) enum OperationCase {
    Add = 1,
    Sub = 2,
    Mul = 3,
    Neg = 4,

    #[allow(non_camel_case_types)]
    not_set = 0
  }
}  // mod Expression_

#[allow(non_camel_case_types)]
// TODO: Implement support for debug redaction
#[derive(Debug)]
pub struct Symbol {
  inner: ::__pb::__runtime::MessageInner
}

// SAFETY:
// - `Symbol` does not provide shared mutation with its arena.
// - `SymbolMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena that would conflict with
//   field access is impossible.
unsafe impl Sync for Symbol {}

impl ::__pb::Proxied for Symbol {
  type View<'a> = SymbolView<'a>;
  type Mut<'a> = SymbolMut<'a>;
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub struct SymbolView<'a> {
  msg: ::__pb::__internal::RawMessage,
  _phantom: ::__std::marker::PhantomData<&'a ()>,
}

impl<'a> SymbolView<'a> {
  #[doc(hidden)]
  pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
    Self { msg, _phantom: std::marker::PhantomData }
  }
  pub fn r#subproofId(&self) -> u32 { unsafe {
    pilout_Symbol_subproofId(self.msg)
  } }

  pub fn r#airId(&self) -> u32 { unsafe {
    pilout_Symbol_airId(self.msg)
  } }

  pub fn r#id(&self) -> u32 { unsafe {
    pilout_Symbol_id(self.msg)
  } }

  pub fn r#stage(&self) -> u32 { unsafe {
    pilout_Symbol_stage(self.msg)
  } }

  pub fn r#dim(&self) -> u32 { unsafe {
    pilout_Symbol_dim(self.msg)
  } }

}

// SAFETY:
// - `SymbolView` does not perform any mutation.
// - While a `SymbolView` exists, a `SymbolMut` can't exist to mutate
//   the arena that would conflict with field access.
// - `SymbolMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for SymbolView<'_> {}
unsafe impl Send for SymbolView<'_> {}

impl<'a> ::__pb::ViewProxy<'a> for SymbolView<'a> {
  type Proxied = Symbol;

  fn as_view(&self) -> ::__pb::View<'a, Symbol> {
    *self
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Symbol> where 'a: 'shorter {
    self
  }
}

impl<'a> ::__pb::SettableValue<Symbol> for SymbolView<'a> {
  fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<Symbol>) {
    todo!()
  }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct SymbolMut<'a> {
  inner: ::__pb::__runtime::MutatorMessageRef<'a>,
}

// SAFETY:
// - `SymbolMut` does not perform any shared mutation.
// - `SymbolMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for SymbolMut<'_> {}

impl<'a> ::__pb::MutProxy<'a> for SymbolMut<'a> {
  fn as_mut(&mut self) -> ::__pb::Mut<'_, Symbol> {
    SymbolMut { inner: self.inner }
  }
  fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, Symbol> where 'a : 'shorter { self }
}

impl<'a> ::__pb::ViewProxy<'a> for SymbolMut<'a> {
  type Proxied = Symbol;
  fn as_view(&self) -> ::__pb::View<'_, Symbol> {
    SymbolView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Symbol> where 'a: 'shorter {
    SymbolView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
}

impl Symbol {
  pub fn new() -> Self {
    let arena = ::__pb::__runtime::Arena::new();
    Self {
      inner: ::__pb::__runtime::MessageInner {
        msg: unsafe { pilout_Symbol_new(arena.raw()) },
        arena,
      }
    }
  }

  pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
    let arena = ::__pb::__runtime::Arena::new();
    let mut len = 0;
    unsafe {
      let data = pilout_Symbol_serialize(self.inner.msg, arena.raw(), &mut len);
      ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
    }
  }
  pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
    let arena = ::__pb::__runtime::Arena::new();
    let msg = unsafe {
      pilout_Symbol_parse(data.as_ptr(), data.len(), arena.raw())
    };

    match msg {
      None => Err(::__pb::ParseError),
      Some(msg) => {
        // This assignment causes self.arena to be dropped and to deallocate
        // any previous message pointed/owned to by self.inner.msg.
        self.inner.arena = arena;
        self.inner.msg = msg;
        Ok(())
      }
    }
  }

  // name: optional string
  pub fn r#name(&self) -> &::__pb::ProtoStr {
    let view = unsafe { pilout_Symbol_name(self.inner.msg).as_ref() };
    // SAFETY: The runtime doesn't require ProtoStr to be UTF-8.
    unsafe { ::__pb::ProtoStr::from_utf8_unchecked(view) }
  }

  pub fn name_mut(&mut self) -> ::__pb::Mut<'_, ::__pb::ProtoStr> {
    static VTABLE: ::__pb::__internal::BytesMutVTable = unsafe {
      ::__pb::__internal::BytesMutVTable::new(
        ::__pb::__internal::Private,
        pilout_Symbol_name,
        pilout_Symbol_set_name,
      )
    };
    unsafe {
      <::__pb::Mut<::__pb::ProtoStr>>::from_inner(
        ::__pb::__internal::Private,
        ::__pb::__internal::RawVTableMutator::new(
          ::__pb::__internal::Private,
          ::__pb::__runtime::MutatorMessageRef::new(
            ::__pb::__internal::Private, &mut self.inner),
          &VTABLE,
        )
      )
    }
  }

  // subproofId: optional uint32
  pub fn r#subproofId(&self) -> u32 {
    unsafe { pilout_Symbol_subproofId(self.inner.msg) }
  }
  pub fn r#subproofId_opt(&self) -> ::__pb::Optional<u32> {
    if !unsafe { pilout_Symbol_has_subproofId(self.inner.msg) } {
      return ::__pb::Optional::Unset(<u32>::default());
    }
    let value = unsafe { pilout_Symbol_subproofId(self.inner.msg) };
    ::__pb::Optional::Set(value)
  }
  pub fn r#subproofId_set(&mut self, val: Option<u32>) {
    match val {
      Some(val) => unsafe { pilout_Symbol_set_subproofId(self.inner.msg, val) },
      None => unsafe { pilout_Symbol_clear_subproofId(self.inner.msg) },
    }
  }

  // airId: optional uint32
  pub fn r#airId(&self) -> u32 {
    unsafe { pilout_Symbol_airId(self.inner.msg) }
  }
  pub fn r#airId_opt(&self) -> ::__pb::Optional<u32> {
    if !unsafe { pilout_Symbol_has_airId(self.inner.msg) } {
      return ::__pb::Optional::Unset(<u32>::default());
    }
    let value = unsafe { pilout_Symbol_airId(self.inner.msg) };
    ::__pb::Optional::Set(value)
  }
  pub fn r#airId_set(&mut self, val: Option<u32>) {
    match val {
      Some(val) => unsafe { pilout_Symbol_set_airId(self.inner.msg, val) },
      None => unsafe { pilout_Symbol_clear_airId(self.inner.msg) },
    }
  }

  // type: optional enum pilout.SymbolType
  // Unsupported! :(


  // id: optional uint32
  pub fn r#id(&self) -> u32 {
    unsafe { pilout_Symbol_id(self.inner.msg) }
  }
  pub fn r#id_mut(&mut self) -> ::__pb::PrimitiveMut<'_, u32> {
    static VTABLE: ::__pb::__internal::PrimitiveVTable<u32> =
      ::__pb::__internal::PrimitiveVTable::new(
        ::__pb::__internal::Private,
        pilout_Symbol_id,
        pilout_Symbol_set_id,
      );

      ::__pb::PrimitiveMut::from_inner(
        ::__pb::__internal::Private,
        unsafe {
          ::__pb::__internal::RawVTableMutator::new(
            ::__pb::__internal::Private,
            ::__pb::__runtime::MutatorMessageRef::new(
              ::__pb::__internal::Private, &mut self.inner
            ),
            &VTABLE,
          )
        },
      )
  }

  // stage: optional uint32
  pub fn r#stage(&self) -> u32 {
    unsafe { pilout_Symbol_stage(self.inner.msg) }
  }
  pub fn r#stage_opt(&self) -> ::__pb::Optional<u32> {
    if !unsafe { pilout_Symbol_has_stage(self.inner.msg) } {
      return ::__pb::Optional::Unset(<u32>::default());
    }
    let value = unsafe { pilout_Symbol_stage(self.inner.msg) };
    ::__pb::Optional::Set(value)
  }
  pub fn r#stage_set(&mut self, val: Option<u32>) {
    match val {
      Some(val) => unsafe { pilout_Symbol_set_stage(self.inner.msg, val) },
      None => unsafe { pilout_Symbol_clear_stage(self.inner.msg) },
    }
  }

  // dim: optional uint32
  pub fn r#dim(&self) -> u32 {
    unsafe { pilout_Symbol_dim(self.inner.msg) }
  }
  pub fn r#dim_mut(&mut self) -> ::__pb::PrimitiveMut<'_, u32> {
    static VTABLE: ::__pb::__internal::PrimitiveVTable<u32> =
      ::__pb::__internal::PrimitiveVTable::new(
        ::__pb::__internal::Private,
        pilout_Symbol_dim,
        pilout_Symbol_set_dim,
      );

      ::__pb::PrimitiveMut::from_inner(
        ::__pb::__internal::Private,
        unsafe {
          ::__pb::__internal::RawVTableMutator::new(
            ::__pb::__internal::Private,
            ::__pb::__runtime::MutatorMessageRef::new(
              ::__pb::__internal::Private, &mut self.inner
            ),
            &VTABLE,
          )
        },
      )
  }

  // lengths: repeated uint32
  // Unsupported! :(


  // debugLine: optional string
  pub fn r#debugLine(&self) -> &::__pb::ProtoStr {
    let view = unsafe { pilout_Symbol_debugLine(self.inner.msg).as_ref() };
    // SAFETY: The runtime doesn't require ProtoStr to be UTF-8.
    unsafe { ::__pb::ProtoStr::from_utf8_unchecked(view) }
  }

  pub fn debugLine_opt(&self) -> ::__pb::Optional<&::__pb::ProtoStr> {
    unsafe {
      let view = pilout_Symbol_debugLine(self.inner.msg).as_ref();
      ::__pb::Optional::new(
        // SAFETY: The runtime doesn't require ProtoStr to be UTF-8.
        unsafe { ::__pb::ProtoStr::from_utf8_unchecked(view) }
         ,
        pilout_Symbol_has_debugLine(self.inner.msg)
      )
    }
  }
  pub fn debugLine_mut(&mut self) -> ::__pb::FieldEntry<'_, ::__pb::ProtoStr> {
    static VTABLE: ::__pb::__internal::BytesOptionalMutVTable = unsafe {
      ::__pb::__internal::BytesOptionalMutVTable::new(
        ::__pb::__internal::Private,
        pilout_Symbol_debugLine,
        pilout_Symbol_set_debugLine,
        pilout_Symbol_clear_debugLine,
        b"",
      )
    };
    let out = unsafe {
      let has = pilout_Symbol_has_debugLine(self.inner.msg);
      ::__pb::__internal::new_vtable_field_entry(
        ::__pb::__internal::Private,
        ::__pb::__runtime::MutatorMessageRef::new(
          ::__pb::__internal::Private, &mut self.inner),
        &VTABLE,
        has,
      )
    };
    ::__pb::ProtoStrMut::field_entry_from_bytes(
      ::__pb::__internal::Private, out
    )
  }


}  // impl Symbol

impl ::__std::ops::Drop for Symbol {
  fn drop(&mut self) {
  }
}

extern "C" {
  fn pilout_Symbol_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
  fn pilout_Symbol_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
  fn pilout_Symbol_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_Symbol_name(raw_msg: ::__pb::__internal::RawMessage) -> ::__pb::__internal::PtrAndLen;
  fn pilout_Symbol_set_name(raw_msg: ::__pb::__internal::RawMessage, val: ::__pb::__internal::PtrAndLen);
  fn pilout_Symbol_clear_name(raw_msg: ::__pb::__internal::RawMessage);

  fn pilout_Symbol_has_subproofId(raw_msg: ::__pb::__internal::RawMessage) -> bool;
  fn pilout_Symbol_subproofId(raw_msg: ::__pb::__internal::RawMessage) -> u32;
  fn pilout_Symbol_set_subproofId(raw_msg: ::__pb::__internal::RawMessage, val: u32);
  fn pilout_Symbol_clear_subproofId(raw_msg: ::__pb::__internal::RawMessage);

  fn pilout_Symbol_has_airId(raw_msg: ::__pb::__internal::RawMessage) -> bool;
  fn pilout_Symbol_airId(raw_msg: ::__pb::__internal::RawMessage) -> u32;
  fn pilout_Symbol_set_airId(raw_msg: ::__pb::__internal::RawMessage, val: u32);
  fn pilout_Symbol_clear_airId(raw_msg: ::__pb::__internal::RawMessage);


  fn pilout_Symbol_id(raw_msg: ::__pb::__internal::RawMessage) -> u32;
  fn pilout_Symbol_set_id(raw_msg: ::__pb::__internal::RawMessage, val: u32);
  fn pilout_Symbol_clear_id(raw_msg: ::__pb::__internal::RawMessage);

  fn pilout_Symbol_has_stage(raw_msg: ::__pb::__internal::RawMessage) -> bool;
  fn pilout_Symbol_stage(raw_msg: ::__pb::__internal::RawMessage) -> u32;
  fn pilout_Symbol_set_stage(raw_msg: ::__pb::__internal::RawMessage, val: u32);
  fn pilout_Symbol_clear_stage(raw_msg: ::__pb::__internal::RawMessage);

  fn pilout_Symbol_dim(raw_msg: ::__pb::__internal::RawMessage) -> u32;
  fn pilout_Symbol_set_dim(raw_msg: ::__pb::__internal::RawMessage, val: u32);
  fn pilout_Symbol_clear_dim(raw_msg: ::__pb::__internal::RawMessage);


  fn pilout_Symbol_has_debugLine(raw_msg: ::__pb::__internal::RawMessage) -> bool;
  fn pilout_Symbol_debugLine(raw_msg: ::__pb::__internal::RawMessage) -> ::__pb::__internal::PtrAndLen;
  fn pilout_Symbol_set_debugLine(raw_msg: ::__pb::__internal::RawMessage, val: ::__pb::__internal::PtrAndLen);
  fn pilout_Symbol_clear_debugLine(raw_msg: ::__pb::__internal::RawMessage);


}  // extern "C" for Symbol


#[allow(non_camel_case_types)]
// TODO: Implement support for debug redaction
#[derive(Debug)]
pub struct HintField {
  inner: ::__pb::__runtime::MessageInner
}

// SAFETY:
// - `HintField` does not provide shared mutation with its arena.
// - `HintFieldMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena that would conflict with
//   field access is impossible.
unsafe impl Sync for HintField {}

impl ::__pb::Proxied for HintField {
  type View<'a> = HintFieldView<'a>;
  type Mut<'a> = HintFieldMut<'a>;
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub struct HintFieldView<'a> {
  msg: ::__pb::__internal::RawMessage,
  _phantom: ::__std::marker::PhantomData<&'a ()>,
}

impl<'a> HintFieldView<'a> {
  #[doc(hidden)]
  pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
    Self { msg, _phantom: std::marker::PhantomData }
  }
}

// SAFETY:
// - `HintFieldView` does not perform any mutation.
// - While a `HintFieldView` exists, a `HintFieldMut` can't exist to mutate
//   the arena that would conflict with field access.
// - `HintFieldMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for HintFieldView<'_> {}
unsafe impl Send for HintFieldView<'_> {}

impl<'a> ::__pb::ViewProxy<'a> for HintFieldView<'a> {
  type Proxied = HintField;

  fn as_view(&self) -> ::__pb::View<'a, HintField> {
    *self
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, HintField> where 'a: 'shorter {
    self
  }
}

impl<'a> ::__pb::SettableValue<HintField> for HintFieldView<'a> {
  fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<HintField>) {
    todo!()
  }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct HintFieldMut<'a> {
  inner: ::__pb::__runtime::MutatorMessageRef<'a>,
}

// SAFETY:
// - `HintFieldMut` does not perform any shared mutation.
// - `HintFieldMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for HintFieldMut<'_> {}

impl<'a> ::__pb::MutProxy<'a> for HintFieldMut<'a> {
  fn as_mut(&mut self) -> ::__pb::Mut<'_, HintField> {
    HintFieldMut { inner: self.inner }
  }
  fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, HintField> where 'a : 'shorter { self }
}

impl<'a> ::__pb::ViewProxy<'a> for HintFieldMut<'a> {
  type Proxied = HintField;
  fn as_view(&self) -> ::__pb::View<'_, HintField> {
    HintFieldView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, HintField> where 'a: 'shorter {
    HintFieldView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
}

impl HintField {
  pub fn new() -> Self {
    let arena = ::__pb::__runtime::Arena::new();
    Self {
      inner: ::__pb::__runtime::MessageInner {
        msg: unsafe { pilout_HintField_new(arena.raw()) },
        arena,
      }
    }
  }

  pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
    let arena = ::__pb::__runtime::Arena::new();
    let mut len = 0;
    unsafe {
      let data = pilout_HintField_serialize(self.inner.msg, arena.raw(), &mut len);
      ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
    }
  }
  pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
    let arena = ::__pb::__runtime::Arena::new();
    let msg = unsafe {
      pilout_HintField_parse(data.as_ptr(), data.len(), arena.raw())
    };

    match msg {
      None => Err(::__pb::ParseError),
      Some(msg) => {
        // This assignment causes self.arena to be dropped and to deallocate
        // any previous message pointed/owned to by self.inner.msg.
        self.inner.arena = arena;
        self.inner.msg = msg;
        Ok(())
      }
    }
  }

  // name: optional string
  pub fn r#name(&self) -> &::__pb::ProtoStr {
    let view = unsafe { pilout_HintField_name(self.inner.msg).as_ref() };
    // SAFETY: The runtime doesn't require ProtoStr to be UTF-8.
    unsafe { ::__pb::ProtoStr::from_utf8_unchecked(view) }
  }

  pub fn name_opt(&self) -> ::__pb::Optional<&::__pb::ProtoStr> {
    unsafe {
      let view = pilout_HintField_name(self.inner.msg).as_ref();
      ::__pb::Optional::new(
        // SAFETY: The runtime doesn't require ProtoStr to be UTF-8.
        unsafe { ::__pb::ProtoStr::from_utf8_unchecked(view) }
         ,
        pilout_HintField_has_name(self.inner.msg)
      )
    }
  }
  pub fn name_mut(&mut self) -> ::__pb::FieldEntry<'_, ::__pb::ProtoStr> {
    static VTABLE: ::__pb::__internal::BytesOptionalMutVTable = unsafe {
      ::__pb::__internal::BytesOptionalMutVTable::new(
        ::__pb::__internal::Private,
        pilout_HintField_name,
        pilout_HintField_set_name,
        pilout_HintField_clear_name,
        b"",
      )
    };
    let out = unsafe {
      let has = pilout_HintField_has_name(self.inner.msg);
      ::__pb::__internal::new_vtable_field_entry(
        ::__pb::__internal::Private,
        ::__pb::__runtime::MutatorMessageRef::new(
          ::__pb::__internal::Private, &mut self.inner),
        &VTABLE,
        has,
      )
    };
    ::__pb::ProtoStrMut::field_entry_from_bytes(
      ::__pb::__internal::Private, out
    )
  }

  // stringValue: optional string
  pub fn r#stringValue(&self) -> &::__pb::ProtoStr {
    let view = unsafe { pilout_HintField_stringValue(self.inner.msg).as_ref() };
    // SAFETY: The runtime doesn't require ProtoStr to be UTF-8.
    unsafe { ::__pb::ProtoStr::from_utf8_unchecked(view) }
  }

  pub fn stringValue_opt(&self) -> ::__pb::Optional<&::__pb::ProtoStr> {
    unsafe {
      let view = pilout_HintField_stringValue(self.inner.msg).as_ref();
      ::__pb::Optional::new(
        // SAFETY: The runtime doesn't require ProtoStr to be UTF-8.
        unsafe { ::__pb::ProtoStr::from_utf8_unchecked(view) }
         ,
        pilout_HintField_has_stringValue(self.inner.msg)
      )
    }
  }
  pub fn stringValue_mut(&mut self) -> ::__pb::FieldEntry<'_, ::__pb::ProtoStr> {
    static VTABLE: ::__pb::__internal::BytesOptionalMutVTable = unsafe {
      ::__pb::__internal::BytesOptionalMutVTable::new(
        ::__pb::__internal::Private,
        pilout_HintField_stringValue,
        pilout_HintField_set_stringValue,
        pilout_HintField_clear_stringValue,
        b"",
      )
    };
    let out = unsafe {
      let has = pilout_HintField_has_stringValue(self.inner.msg);
      ::__pb::__internal::new_vtable_field_entry(
        ::__pb::__internal::Private,
        ::__pb::__runtime::MutatorMessageRef::new(
          ::__pb::__internal::Private, &mut self.inner),
        &VTABLE,
        has,
      )
    };
    ::__pb::ProtoStrMut::field_entry_from_bytes(
      ::__pb::__internal::Private, out
    )
  }

  // operand: optional message pilout.Operand
  pub fn r#operand(&self) -> crate::pilout::OperandView {
    let submsg = unsafe { pilout_HintField_operand(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::OperandView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::OperandView::new(::__pb::__internal::Private, field),
      }
  }

  // hintFieldArray: optional message pilout.HintFieldArray
  pub fn r#hintFieldArray(&self) -> crate::pilout::HintFieldArrayView {
    let submsg = unsafe { pilout_HintField_hintFieldArray(self.inner.msg) };
    // For upb, getters return null if the field is unset, so we need to
    // check for null and return the default instance manually. Note that
    // a null ptr received from upb manifests as Option::None
    match submsg {
        // TODO:(b/304357029)
        None => crate::pilout::HintFieldArrayView::new(::__pb::__internal::Private, ::__pb::__runtime::ScratchSpace::zeroed_block()),
        Some(field) => crate::pilout::HintFieldArrayView::new(::__pb::__internal::Private, field),
      }
  }


  pub fn r#value(&self) -> HintField_::Value {
    match unsafe { pilout_HintField_value_case(self.inner.msg) } {
      _ => HintField_::Value::not_set(std::marker::PhantomData)
    }
  }

  pub fn r#value_mut(&mut self) -> HintField_::ValueMut {
    match unsafe { pilout_HintField_value_case(self.inner.msg) } {
      _ => HintField_::ValueMut::not_set(std::marker::PhantomData)
    }
  }

}  // impl HintField

impl ::__std::ops::Drop for HintField {
  fn drop(&mut self) {
  }
}

extern "C" {
  fn pilout_HintField_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
  fn pilout_HintField_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
  fn pilout_HintField_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_HintField_has_name(raw_msg: ::__pb::__internal::RawMessage) -> bool;
  fn pilout_HintField_name(raw_msg: ::__pb::__internal::RawMessage) -> ::__pb::__internal::PtrAndLen;
  fn pilout_HintField_set_name(raw_msg: ::__pb::__internal::RawMessage, val: ::__pb::__internal::PtrAndLen);
  fn pilout_HintField_clear_name(raw_msg: ::__pb::__internal::RawMessage);

  fn pilout_HintField_has_stringValue(raw_msg: ::__pb::__internal::RawMessage) -> bool;
  fn pilout_HintField_stringValue(raw_msg: ::__pb::__internal::RawMessage) -> ::__pb::__internal::PtrAndLen;
  fn pilout_HintField_set_stringValue(raw_msg: ::__pb::__internal::RawMessage, val: ::__pb::__internal::PtrAndLen);
  fn pilout_HintField_clear_stringValue(raw_msg: ::__pb::__internal::RawMessage);

  fn pilout_HintField_operand(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_HintField_hintFieldArray(raw_msg: ::__pb::__internal::RawMessage) -> Option<::__pb::__internal::RawMessage>;


  fn pilout_HintField_value_case(raw_msg: ::__pb::__internal::RawMessage) -> HintField_::ValueCase;

}  // extern "C" for HintField

#[allow(non_snake_case)]
pub mod HintField_ {

  #[non_exhaustive]
  #[derive(Debug)]
  #[allow(dead_code)]
  #[repr(isize)]
  pub enum Value<'msg> {

    #[allow(non_camel_case_types)]
    not_set(std::marker::PhantomData<&'msg ()>) = 0
  }

  #[non_exhaustive]
  #[derive(Debug)]
  #[allow(dead_code)]
  #[repr(isize)]
  pub enum ValueMut<'msg> {

    #[allow(non_camel_case_types)]
    not_set(std::marker::PhantomData<&'msg ()>) = 0
  }
  #[repr(C)]
  #[derive(Debug, Copy, Clone, PartialEq, Eq)]
  pub(super) enum ValueCase {
    StringValue = 2,
    Operand = 3,
    HintFieldArray = 4,

    #[allow(non_camel_case_types)]
    not_set = 0
  }
}  // mod HintField_

#[allow(non_camel_case_types)]
// TODO: Implement support for debug redaction
#[derive(Debug)]
pub struct HintFieldArray {
  inner: ::__pb::__runtime::MessageInner
}

// SAFETY:
// - `HintFieldArray` does not provide shared mutation with its arena.
// - `HintFieldArrayMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena that would conflict with
//   field access is impossible.
unsafe impl Sync for HintFieldArray {}

impl ::__pb::Proxied for HintFieldArray {
  type View<'a> = HintFieldArrayView<'a>;
  type Mut<'a> = HintFieldArrayMut<'a>;
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub struct HintFieldArrayView<'a> {
  msg: ::__pb::__internal::RawMessage,
  _phantom: ::__std::marker::PhantomData<&'a ()>,
}

impl<'a> HintFieldArrayView<'a> {
  #[doc(hidden)]
  pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
    Self { msg, _phantom: std::marker::PhantomData }
  }
}

// SAFETY:
// - `HintFieldArrayView` does not perform any mutation.
// - While a `HintFieldArrayView` exists, a `HintFieldArrayMut` can't exist to mutate
//   the arena that would conflict with field access.
// - `HintFieldArrayMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for HintFieldArrayView<'_> {}
unsafe impl Send for HintFieldArrayView<'_> {}

impl<'a> ::__pb::ViewProxy<'a> for HintFieldArrayView<'a> {
  type Proxied = HintFieldArray;

  fn as_view(&self) -> ::__pb::View<'a, HintFieldArray> {
    *self
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, HintFieldArray> where 'a: 'shorter {
    self
  }
}

impl<'a> ::__pb::SettableValue<HintFieldArray> for HintFieldArrayView<'a> {
  fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<HintFieldArray>) {
    todo!()
  }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct HintFieldArrayMut<'a> {
  inner: ::__pb::__runtime::MutatorMessageRef<'a>,
}

// SAFETY:
// - `HintFieldArrayMut` does not perform any shared mutation.
// - `HintFieldArrayMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for HintFieldArrayMut<'_> {}

impl<'a> ::__pb::MutProxy<'a> for HintFieldArrayMut<'a> {
  fn as_mut(&mut self) -> ::__pb::Mut<'_, HintFieldArray> {
    HintFieldArrayMut { inner: self.inner }
  }
  fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, HintFieldArray> where 'a : 'shorter { self }
}

impl<'a> ::__pb::ViewProxy<'a> for HintFieldArrayMut<'a> {
  type Proxied = HintFieldArray;
  fn as_view(&self) -> ::__pb::View<'_, HintFieldArray> {
    HintFieldArrayView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, HintFieldArray> where 'a: 'shorter {
    HintFieldArrayView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
}

impl HintFieldArray {
  pub fn new() -> Self {
    let arena = ::__pb::__runtime::Arena::new();
    Self {
      inner: ::__pb::__runtime::MessageInner {
        msg: unsafe { pilout_HintFieldArray_new(arena.raw()) },
        arena,
      }
    }
  }

  pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
    let arena = ::__pb::__runtime::Arena::new();
    let mut len = 0;
    unsafe {
      let data = pilout_HintFieldArray_serialize(self.inner.msg, arena.raw(), &mut len);
      ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
    }
  }
  pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
    let arena = ::__pb::__runtime::Arena::new();
    let msg = unsafe {
      pilout_HintFieldArray_parse(data.as_ptr(), data.len(), arena.raw())
    };

    match msg {
      None => Err(::__pb::ParseError),
      Some(msg) => {
        // This assignment causes self.arena to be dropped and to deallocate
        // any previous message pointed/owned to by self.inner.msg.
        self.inner.arena = arena;
        self.inner.msg = msg;
        Ok(())
      }
    }
  }

  // hintFields: repeated message pilout.HintField
  // Unsupported! :(



}  // impl HintFieldArray

impl ::__std::ops::Drop for HintFieldArray {
  fn drop(&mut self) {
  }
}

extern "C" {
  fn pilout_HintFieldArray_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
  fn pilout_HintFieldArray_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
  fn pilout_HintFieldArray_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;



}  // extern "C" for HintFieldArray


#[allow(non_camel_case_types)]
// TODO: Implement support for debug redaction
#[derive(Debug)]
pub struct Hint {
  inner: ::__pb::__runtime::MessageInner
}

// SAFETY:
// - `Hint` does not provide shared mutation with its arena.
// - `HintMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena that would conflict with
//   field access is impossible.
unsafe impl Sync for Hint {}

impl ::__pb::Proxied for Hint {
  type View<'a> = HintView<'a>;
  type Mut<'a> = HintMut<'a>;
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub struct HintView<'a> {
  msg: ::__pb::__internal::RawMessage,
  _phantom: ::__std::marker::PhantomData<&'a ()>,
}

impl<'a> HintView<'a> {
  #[doc(hidden)]
  pub fn new(_private: ::__pb::__internal::Private, msg: ::__pb::__internal::RawMessage) -> Self {
    Self { msg, _phantom: std::marker::PhantomData }
  }
  pub fn r#subproofId(&self) -> u32 { unsafe {
    pilout_Hint_subproofId(self.msg)
  } }

  pub fn r#airId(&self) -> u32 { unsafe {
    pilout_Hint_airId(self.msg)
  } }

}

// SAFETY:
// - `HintView` does not perform any mutation.
// - While a `HintView` exists, a `HintMut` can't exist to mutate
//   the arena that would conflict with field access.
// - `HintMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for HintView<'_> {}
unsafe impl Send for HintView<'_> {}

impl<'a> ::__pb::ViewProxy<'a> for HintView<'a> {
  type Proxied = Hint;

  fn as_view(&self) -> ::__pb::View<'a, Hint> {
    *self
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Hint> where 'a: 'shorter {
    self
  }
}

impl<'a> ::__pb::SettableValue<Hint> for HintView<'a> {
  fn set_on(self, _private: ::__pb::__internal::Private, _mutator: ::__pb::Mut<Hint>) {
    todo!()
  }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct HintMut<'a> {
  inner: ::__pb::__runtime::MutatorMessageRef<'a>,
}

// SAFETY:
// - `HintMut` does not perform any shared mutation.
// - `HintMut` is not `Send`, and so even in the presence of mutator
//   splitting, synchronous access of an arena is impossible.
unsafe impl Sync for HintMut<'_> {}

impl<'a> ::__pb::MutProxy<'a> for HintMut<'a> {
  fn as_mut(&mut self) -> ::__pb::Mut<'_, Hint> {
    HintMut { inner: self.inner }
  }
  fn into_mut<'shorter>(self) -> ::__pb::Mut<'shorter, Hint> where 'a : 'shorter { self }
}

impl<'a> ::__pb::ViewProxy<'a> for HintMut<'a> {
  type Proxied = Hint;
  fn as_view(&self) -> ::__pb::View<'_, Hint> {
    HintView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
  fn into_view<'shorter>(self) -> ::__pb::View<'shorter, Hint> where 'a: 'shorter {
    HintView { msg: self.inner.msg(), _phantom: std::marker::PhantomData }
  }
}

impl Hint {
  pub fn new() -> Self {
    let arena = ::__pb::__runtime::Arena::new();
    Self {
      inner: ::__pb::__runtime::MessageInner {
        msg: unsafe { pilout_Hint_new(arena.raw()) },
        arena,
      }
    }
  }

  pub fn serialize(&self) -> ::__pb::__runtime::SerializedData {
    let arena = ::__pb::__runtime::Arena::new();
    let mut len = 0;
    unsafe {
      let data = pilout_Hint_serialize(self.inner.msg, arena.raw(), &mut len);
      ::__pb::__runtime::SerializedData::from_raw_parts(arena, data, len)
    }
  }
  pub fn deserialize(&mut self, data: &[u8]) -> Result<(), ::__pb::ParseError> {
    let arena = ::__pb::__runtime::Arena::new();
    let msg = unsafe {
      pilout_Hint_parse(data.as_ptr(), data.len(), arena.raw())
    };

    match msg {
      None => Err(::__pb::ParseError),
      Some(msg) => {
        // This assignment causes self.arena to be dropped and to deallocate
        // any previous message pointed/owned to by self.inner.msg.
        self.inner.arena = arena;
        self.inner.msg = msg;
        Ok(())
      }
    }
  }

  // name: optional string
  pub fn r#name(&self) -> &::__pb::ProtoStr {
    let view = unsafe { pilout_Hint_name(self.inner.msg).as_ref() };
    // SAFETY: The runtime doesn't require ProtoStr to be UTF-8.
    unsafe { ::__pb::ProtoStr::from_utf8_unchecked(view) }
  }

  pub fn name_mut(&mut self) -> ::__pb::Mut<'_, ::__pb::ProtoStr> {
    static VTABLE: ::__pb::__internal::BytesMutVTable = unsafe {
      ::__pb::__internal::BytesMutVTable::new(
        ::__pb::__internal::Private,
        pilout_Hint_name,
        pilout_Hint_set_name,
      )
    };
    unsafe {
      <::__pb::Mut<::__pb::ProtoStr>>::from_inner(
        ::__pb::__internal::Private,
        ::__pb::__internal::RawVTableMutator::new(
          ::__pb::__internal::Private,
          ::__pb::__runtime::MutatorMessageRef::new(
            ::__pb::__internal::Private, &mut self.inner),
          &VTABLE,
        )
      )
    }
  }

  // hintFields: repeated message pilout.HintField
  // Unsupported! :(


  // subproofId: optional uint32
  pub fn r#subproofId(&self) -> u32 {
    unsafe { pilout_Hint_subproofId(self.inner.msg) }
  }
  pub fn r#subproofId_opt(&self) -> ::__pb::Optional<u32> {
    if !unsafe { pilout_Hint_has_subproofId(self.inner.msg) } {
      return ::__pb::Optional::Unset(<u32>::default());
    }
    let value = unsafe { pilout_Hint_subproofId(self.inner.msg) };
    ::__pb::Optional::Set(value)
  }
  pub fn r#subproofId_set(&mut self, val: Option<u32>) {
    match val {
      Some(val) => unsafe { pilout_Hint_set_subproofId(self.inner.msg, val) },
      None => unsafe { pilout_Hint_clear_subproofId(self.inner.msg) },
    }
  }

  // airId: optional uint32
  pub fn r#airId(&self) -> u32 {
    unsafe { pilout_Hint_airId(self.inner.msg) }
  }
  pub fn r#airId_opt(&self) -> ::__pb::Optional<u32> {
    if !unsafe { pilout_Hint_has_airId(self.inner.msg) } {
      return ::__pb::Optional::Unset(<u32>::default());
    }
    let value = unsafe { pilout_Hint_airId(self.inner.msg) };
    ::__pb::Optional::Set(value)
  }
  pub fn r#airId_set(&mut self, val: Option<u32>) {
    match val {
      Some(val) => unsafe { pilout_Hint_set_airId(self.inner.msg, val) },
      None => unsafe { pilout_Hint_clear_airId(self.inner.msg) },
    }
  }


}  // impl Hint

impl ::__std::ops::Drop for Hint {
  fn drop(&mut self) {
  }
}

extern "C" {
  fn pilout_Hint_new(arena: ::__pb::__internal::RawArena) -> ::__pb::__internal::RawMessage;
  fn pilout_Hint_serialize(msg: ::__pb::__internal::RawMessage, arena: ::__pb::__internal::RawArena, len: &mut usize) -> ::__std::ptr::NonNull<u8>;
  fn pilout_Hint_parse(data: *const u8, size: usize, arena: ::__pb::__internal::RawArena) -> Option<::__pb::__internal::RawMessage>;

  fn pilout_Hint_name(raw_msg: ::__pb::__internal::RawMessage) -> ::__pb::__internal::PtrAndLen;
  fn pilout_Hint_set_name(raw_msg: ::__pb::__internal::RawMessage, val: ::__pb::__internal::PtrAndLen);
  fn pilout_Hint_clear_name(raw_msg: ::__pb::__internal::RawMessage);


  fn pilout_Hint_has_subproofId(raw_msg: ::__pb::__internal::RawMessage) -> bool;
  fn pilout_Hint_subproofId(raw_msg: ::__pb::__internal::RawMessage) -> u32;
  fn pilout_Hint_set_subproofId(raw_msg: ::__pb::__internal::RawMessage, val: u32);
  fn pilout_Hint_clear_subproofId(raw_msg: ::__pb::__internal::RawMessage);

  fn pilout_Hint_has_airId(raw_msg: ::__pb::__internal::RawMessage) -> bool;
  fn pilout_Hint_airId(raw_msg: ::__pb::__internal::RawMessage) -> u32;
  fn pilout_Hint_set_airId(raw_msg: ::__pb::__internal::RawMessage, val: u32);
  fn pilout_Hint_clear_airId(raw_msg: ::__pb::__internal::RawMessage);


}  // extern "C" for Hint


} // mod pilout
