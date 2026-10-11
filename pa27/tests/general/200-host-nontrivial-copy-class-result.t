// Host-object ABI: a class with a non-trivial copy or move constructor is
// returned through caller storage in both directions, even when it fits in
// registers and its move constructor is trivial.
