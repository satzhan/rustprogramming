# Generics & Traits


**View:** https://satzhan.github.io/rustprogramming/Module6Generics_Trait/generics-and-traits.html

## What it covers

Twelve interactive sections in two parts.

**Part I — Generics:** the move from duplicated functions to one generic; trait bounds (`PartialOrd`, `Copy`) shown through the `largest` function; generic structs with one and two type parameters; generic methods, including methods that introduce their own parameter; a `Stack<T>` that needs no bounds at all; and turbofish for when inference runs out.

**Part II — Traits:** one shared ability across unlike types; the three equivalent spellings of a bound (`<T: Trait>`, `impl Trait`, and `where`); default methods; the orphan rule and the newtype workaround; static versus dynamic dispatch at the which-to-use level; and a generic `Cache<K, V>` whose `Hash + Eq` bounds tie back to where bounds were introduced.

