# Writing a Custom Allocator in Rust

The global allocator in Rust is convenient until it isn't. In WASM contexts especially, the default allocator brings in more machinery than many use cases demand. This post walks through implementing a simple bump allocator as a `GlobalAlloc` implementor, and explores where custom allocators make sense.

## What is a bump allocator?

A bump allocator maintains a single pointer into a memory region. Every allocation simply advances that pointer by the requested size (plus alignment padding). Deallocation is a no-op — memory is only reclaimed all at once when the entire arena is reset.

```rust
use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};

pub struct BumpAllocator<const N: usize> {
    arena: UnsafeCell<[u8; N]>,
    head: AtomicUsize,
}

unsafe impl<const N: usize> Sync for BumpAllocator<N> {}

unsafe impl<const N: usize> GlobalAlloc for BumpAllocator<N> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let arena = self.arena.get();
        let start = (*arena).as_mut_ptr() as usize;
        let mut current = self.head.load(Ordering::Relaxed);

        loop {
            let align = layout.align();
            let aligned = (start + current + align - 1) & !(align - 1);
            let offset = aligned - start;
            let new_head = offset + layout.size();

            if new_head > N {
                return core::ptr::null_mut();
            }

            match self.head.compare_exchange_weak(
                current,
                new_head,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => return (start + offset) as *mut u8,
                Err(head) => current = head,
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Intentional no-op.
    }
}
```

## When should you actually use this?

The bump allocator shines in scenarios where:

1. **Lifetime is bounded** — you process a request, then throw everything away.
2. **Fragmentation is unacceptable** — embedded targets with tiny heap budgets.
3. **Allocation frequency is high** — request handlers, parsers, deserializers.

In WASM, where the total memory is capped and the GC does not exist, an arena model combined with explicit resets can significantly reduce peak memory usage compared to a general-purpose allocator.

## Caveats

- No individual deallocation: drop implementations still run, but memory is not returned to the arena.
- Thread safety requires careful atomic design or a lock if running in threaded WASM.
- You must know your arena size at compile time (or use a runtime-allocated backing buffer).

For most WASM tools, this tradeoff is entirely acceptable. The `wee_alloc` crate (now archived) took a similar approach; today `talc` and `dlmalloc` are the common alternatives worth evaluating first.

---

*Next: exploring allocator combinators — wrapping a bump allocator with a fallback to the system allocator.*
