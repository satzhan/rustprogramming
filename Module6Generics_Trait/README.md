# Generics & Traits

An interactive single-page walkthrough of Rust generics and traits. One self-contained HTML file — no build step, no dependencies beyond two web fonts. It opens in any browser and publishes on GitHub Pages as a static file.

**View:** https://satzhan.github.io/rustprogramming/Module5GenericsTraits/generics-and-traits.html

**Fallback (before Pages is live):** https://htmlpreview.github.io/?https://github.com/satzhan/rustprogramming/blob/main/Module5GenericsTraits/generics-and-traits.html

## What it covers

Twelve interactive sections in two parts.

**Part I — Generics:** the move from duplicated functions to one generic; trait bounds (`PartialOrd`, `Copy`) shown through the `largest` function; generic structs with one and two type parameters; generic methods, including methods that introduce their own parameter; a `Stack<T>` that needs no bounds at all; and turbofish for when inference runs out.

**Part II — Traits:** one shared ability across unlike types; the three equivalent spellings of a bound (`<T: Trait>`, `impl Trait`, and `where`); default methods; the orphan rule and the newtype workaround; static versus dynamic dispatch at the which-to-use level; and a generic `Cache<K, V>` whose `Hash + Eq` bounds tie back to where bounds were introduced.

Every control is live — bounds toggle between real compiler errors and a clean compile, the stack pushes and pops, the cache stores and reads, and so on.

## Scope

In scope: the mechanics above, pitched for an introductory university course. Deliberately out of scope: associated types, generic associated types, the typestate pattern, async traits, and dispatch internals such as vtables and monomorphized code size.

## Notes

Every code snippet and every compiler error message shown on the page was run through the Rust compiler before inclusion; runtime output is shown, not asserted. Two corrections to earlier course material are folded in: the `largest` failure is attributed to its trait bounds rather than to lifetimes, and implementing a trait on a built-in type is explained through the orphan rule rather than the claim that every type is a struct.

The file is static — no toolchain, server, or build is needed to view or host it.
