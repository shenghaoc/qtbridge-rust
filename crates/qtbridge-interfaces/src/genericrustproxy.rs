// Copyright (C) 2026 The Qt Company Ltd.
// SPDX-License-Identifier: LicenseRef-Qt-Commercial OR LGPL-3.0-only

use crate::{RustObjAccess, call_rust_trait_impl, call_cpp_impl};
use qtbridge_runtime::qproxies::{QRustProxy, QCppProxy, PlacementAddress};
use qtbridge_runtime::{DispatchMetaCall, DynamicMetaObjectData};
use qtbridge_type_lib::QVariant;
use std::cell::RefCell;
use std::rc::Rc;

pub struct GenericRustProxy<CppProxy: QCppProxy, Adapter: ?Sized> {
    pub(crate) cpp_proxy: *mut CppProxy,
    pub(crate) rust_obj: RustObjAccess<Adapter>,
    pub(crate) on_drop: Box<dyn FnOnce()>,
}

impl<CppProxy, Adapter> QRustProxy for GenericRustProxy<CppProxy, Adapter>
where
    CppProxy: QCppProxy<ProxyRustType = Self>,
    Adapter: ?Sized + DispatchMetaCall,
{
    type ProxyCppType = CppProxy;
    type AdapterType = Adapter;

    fn new(
        rust_obj: &Rc<RefCell<Adapter>>,
        metatype: &'static DynamicMetaObjectData,
        at_address: Option<PlacementAddress>,
        on_drop: Box<dyn FnOnce() + 'static>,
    ) -> *mut Self {
        let boxed_self = Box::new(Self {
            cpp_proxy: std::ptr::null_mut(),
            rust_obj: RustObjAccess::new(rust_obj.clone()),
            on_drop,
        });
        let raw_self = Box::into_raw(boxed_self);
        unsafe {
            (*raw_self).cpp_proxy = match at_address {
                Some(addr) => CppProxy::create_at(raw_self, metatype, addr),
                None => CppProxy::create(raw_self, metatype),
            }
        };
        raw_self
    }

    fn get_cpp_proxy(&self) -> *const CppProxy {
        self.cpp_proxy
    }

    fn get_cpp_proxy_mut(&self) -> *mut CppProxy {
        self.cpp_proxy
    }

    fn emit_signal(&self, mut_ref: &mut Adapter, signal_name: &str, argv: &[*const u8]) {
        call_cpp_impl!(mut self, mut_ref, emit_signal(signal_name, argv))
    }

    fn with_rust_ref<R, F: FnOnce(&Adapter) -> R>(&self, f: F) -> R {
        self.rust_obj
            .try_call_rust_with_handle(|adapter| f(adapter))
            .expect("Failed to access Rust object via shared handle")
    }

    fn with_rust_ref_mut<R, F: FnOnce(&mut Adapter) -> R>(&self, f: F) -> R {
        self.rust_obj
            .try_call_rust_with_handle_mut(|adapter| f(adapter))
            .expect("Failed to access Rust object via mutable handle")
    }

    fn get_rust_object_rc(&self) -> Rc<RefCell<Adapter>> {
        self.rust_obj.get_rc()
    }
}

impl<CppProxy, Adapter> GenericRustProxy<CppProxy, Adapter>
where
    CppProxy: QCppProxy<ProxyRustType = Self>,
    Adapter: ?Sized + DispatchMetaCall,
{
    /// Drops the instance pointed to by `self_ptr`.
    ///
    /// Intended to be called exclusively from the paired C++ destructor.
    ///
    /// # Safety
    ///
    /// `self_ptr` must be a pointer previously returned by [`QRustProxy::new`].
    /// It must not have been dropped already.
    pub unsafe fn drop_self(self_ptr: *mut Self) {
        let boxed_self = unsafe { Box::from_raw(self_ptr) };
        (boxed_self.on_drop)();
    }

    pub fn invoke_slot(&self, slot_id: u32, inputs: &[*const u8], outputs: &[*mut u8]) {
        call_rust_trait_impl!(unsafe self, invoke_slot(slot_id, inputs, outputs))
    }

    // `&self` deliberately: mutability of the user object is interior (`RustObjAccess`)
    pub fn invoke_slot_mut(&self, slot_id: u32, inputs: &[*const u8], outputs: &[*mut u8]) {
        call_rust_trait_impl!(unsafe mut self, invoke_slot_mut(slot_id, inputs, outputs))
    }

    pub fn read_property(&self, prop_id: u32) -> QVariant {
        call_rust_trait_impl!(unsafe self, read_property(prop_id))
    }

    // `&self` deliberately: mutability of the user object is interior (`RustObjAccess`)
    pub fn write_property(&self, prop_id: u32, value: &QVariant) {
        call_rust_trait_impl!(unsafe mut self, write_property(prop_id, value))
    }

}

#[macro_export]
macro_rules! impl_qcpp_proxy {
    ($cpp_proxy:ty, $rust_proxy:ty) => {
        impl QCppProxy for $cpp_proxy {
            type ProxyRustType = $rust_proxy;
            fn get_static_meta_object() -> &'static QMetaObject {
                Self::static_qmeta_object()
            }
            fn get_size() -> usize {
                Self::size_of()
            }
            fn get_align() -> usize {
                Self::align_of()
            }
            fn parser_status_cast() -> i32 {
                Self::parser_status_cast()
            }
            unsafe fn create(rust_proxy: *mut Self::ProxyRustType, metaobject: &'static DynamicMetaObjectData) -> *mut Self {
                unsafe { Self::create(rust_proxy, metaobject) }
            }
            unsafe fn create_at(rust_proxy: *mut Self::ProxyRustType, metaobject: &'static DynamicMetaObjectData, addr: *mut u8) -> *mut Self {
                unsafe { Self::create_at(rust_proxy, metaobject, addr) }
            }
            fn emit_signal(self: std::pin::Pin<&mut Self>, signal_name: &str, argv: &[*const u8]) {
                self.emit_signal_cpp(signal_name, argv)
            }
        }
    };
}

/// Pure-Rust model of the synchronous Qt re-entry that model notifications
/// trigger (`beginInsertRows()` -> `rowCount()`), driven through the real
/// `GenericRustProxy`, `RustObjAccess` and `call_cpp_impl!` /
/// `call_rust_trait_impl!`. The "C++" proxy is a Rust type that re-enters the
/// Rust proxy through a raw pointer, exactly as the CXX shims do.
///
/// These tests also pass natively with an exclusive proxy receiver, because
/// whether the optimizer exploits it is not specified. Run them under Miri
/// (`cargo miri test -p qtbridge-interfaces --lib`, with or without
/// `-Zmiri-tree-borrows`) to check the aliasing contract: a `&mut` proxy
/// receiver in the wrappers below is reported as undefined behavior there.
///
/// The `FakeProxy` wrappers mirror, but are not, the `base_*` / `set_data`
/// methods of the model proxies; the Qt quick tests (`tst_qlistmodel` & co.)
/// cover those.
#[cfg(test)]
mod reentrancy_tests {
    use super::*;
    use qtbridge_runtime::qproxies::QCppProxy;
    use qtbridge_type_lib::QMetaObject;
    use std::pin::Pin;

    trait FakeAdapter: DispatchMetaCall + 'static {
        fn row_count(&self) -> i32;
        fn insert_row(&mut self);
        fn set_data(&mut self);
    }

    struct Model {
        rows: i32,
        proxy: *const FakeProxy,
    }

    type FakeProxy = GenericRustProxy<FakeCpp, dyn FakeAdapter>;

    struct FakeCpp {
        rust_proxy: *const FakeProxy,
        seen_row_counts: Vec<i32>,
    }

    unsafe impl DispatchMetaCall for Model {
        unsafe fn invoke_slot(&self, _: u32, _: &[*const u8], _: &[*mut u8]) {}
        // A `&mut self` slot, like `Backend::add_string` in minimal_app.
        unsafe fn invoke_slot_mut(&mut self, _: u32, _: &[*const u8], _: &[*mut u8]) {
            self.insert_row();
        }
        unsafe fn read_property(&self, _: u32) -> QVariant {
            QVariant::default()
        }
        unsafe fn write_property(&mut self, _: u32, _: &QVariant) {}
    }

    impl FakeAdapter for Model {
        fn row_count(&self) -> i32 {
            self.rows
        }
        // Shape of `QListModelBase::push`.
        fn insert_row(&mut self) {
            let proxy = unsafe { &*self.proxy };
            proxy.base_begin_insert_rows(&mut *self);
            self.rows += 1;
        }
        // Shape of `QListModelAdapter::set_data`.
        fn set_data(&mut self) {
            let proxy = unsafe { &*self.proxy };
            proxy.base_data_changed(&mut *self);
        }
    }

    impl FakeProxy {
        // Inbound, like `QListModelProxyRust::row_count`.
        fn row_count(&self) -> i32 {
            call_rust_trait_impl!(self, row_count())
        }
        // Inbound, like `QListModelProxyRust::set_data`.
        fn set_data(&self) {
            call_rust_trait_impl!(mut self, set_data())
        }
        // Outbound, like `QListModelProxyRust::base_begin_insert_rows`.
        fn base_begin_insert_rows(&self, mut_ref: &mut dyn FakeAdapter) {
            call_cpp_impl!(mut self, mut_ref, begin_insert_rows())
        }
        // Outbound, like `QListModelProxyRust::base_data_changed`.
        fn base_data_changed(&self, mut_ref: &mut dyn FakeAdapter) {
            call_cpp_impl!(mut self, mut_ref, data_changed())
        }
    }

    impl FakeCpp {
        fn reenter(&mut self) {
            // Qt calls rowCount() synchronously from within the notification.
            let proxy = unsafe { &*self.rust_proxy };
            let count = proxy.row_count();
            self.seen_row_counts.push(count);
        }
        fn begin_insert_rows(mut self: Pin<&mut Self>) {
            self.reenter();
        }
        fn data_changed(mut self: Pin<&mut Self>) {
            self.reenter();
        }
    }

    impl QCppProxy for FakeCpp {
        type ProxyRustType = FakeProxy;
        fn get_static_meta_object() -> &'static QMetaObject {
            unimplemented!()
        }
        fn get_size() -> usize {
            unimplemented!()
        }
        fn get_align() -> usize {
            unimplemented!()
        }
        fn parser_status_cast() -> i32 {
            unimplemented!()
        }
        unsafe fn create(_: *mut FakeProxy, _: &'static DynamicMetaObjectData) -> *mut Self {
            unimplemented!()
        }
        unsafe fn create_at(_: *mut FakeProxy, _: &'static DynamicMetaObjectData, _: *mut u8) -> *mut Self {
            unimplemented!()
        }
        fn emit_signal(self: Pin<&mut Self>, _: &str, _: &[*const u8]) {}
    }

    /// Returns the raw proxy, as C++ holds it. Free it with `destroy`.
    fn create_proxy(rows: i32) -> *mut FakeProxy {
        let model = Rc::new(RefCell::new(Model { rows, proxy: std::ptr::null() }));
        let adapter: Rc<RefCell<dyn FakeAdapter>> = model.clone();
        let proxy = Box::into_raw(Box::new(FakeProxy {
            cpp_proxy: std::ptr::null_mut(),
            rust_obj: RustObjAccess::new(adapter),
            on_drop: Box::new(|| {}),
        }));
        model.borrow_mut().proxy = proxy;
        let cpp = Box::into_raw(Box::new(FakeCpp { rust_proxy: proxy, seen_row_counts: Vec::new() }));
        unsafe { (*proxy).cpp_proxy = cpp };
        proxy
    }

    fn destroy(proxy: *mut FakeProxy) -> Vec<i32> {
        let cpp = unsafe { Box::from_raw((*proxy).cpp_proxy) };
        unsafe { FakeProxy::drop_self(proxy) };
        cpp.seen_row_counts
    }

    #[test]
    fn require_that_row_count_can_reenter_during_begin_insert_rows_of_a_mutable_slot() {
        let proxy = create_proxy(3);
        // C++ -> Rust mutable slot -> notification -> C++ -> rowCount()
        unsafe { &*proxy }.invoke_slot_mut(0, &[], &[]);
        unsafe { &*proxy }.invoke_slot_mut(0, &[], &[]);
        assert_eq!(unsafe { &*proxy }.row_count(), 5);
        assert_eq!(destroy(proxy), vec![3, 4]);
    }

    #[test]
    fn require_that_row_count_can_reenter_during_data_changed_of_set_data() {
        let proxy = create_proxy(2);
        // C++ -> Rust setData -> notification -> C++ -> rowCount()
        unsafe { &*proxy }.set_data();
        assert_eq!(destroy(proxy), vec![2]);
    }
}
