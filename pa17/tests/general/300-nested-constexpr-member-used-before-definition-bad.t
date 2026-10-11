// N3485 9.2p2, 5.19p2: box<>'s member function bodies are a complete-class
// context of outer; at the static_assert operator bool is an undefined
// constexpr function, not a constant expression; expect reject
struct outer {
	template <int P = 1> struct box { constexpr operator bool() const { return P == 1; } };
	static_assert(box<>(), "not yet defined");
};
int main() { return 0; }
