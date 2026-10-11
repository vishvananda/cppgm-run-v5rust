// N3485 15.2p1/12.2p4: when constructing Q a[3] (Q's default ctor has a
// by-value default argument P(7)) throws on a later element, the
// already-constructed local `before` is still destroyed
