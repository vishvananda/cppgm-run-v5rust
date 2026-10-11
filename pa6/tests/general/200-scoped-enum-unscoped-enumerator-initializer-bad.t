// N3485 7.2p5: without a fixed underlying type an enumerator initializer is
// an integral constant expression; a scoped enumerator converts to no
// integral type implicitly (7.2p10).
enum class A { X = 1 };
enum E { Y = A::X };
