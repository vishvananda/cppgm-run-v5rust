// A declaration-only exception query preserves a defaulted source's raw
// constructor facts until its plan is demanded. Expect exit 0.
struct Member { Member() {} };
template<class T> struct Base {
    Member member;
    Base() = default;
};
struct Derived : Base<int> {
    using Base<int>::Base;
    Derived(Derived&&) = default;
};
struct Owner { Derived value; Owner(); };
Owner::Owner() {}
int main() { Owner value; return 0; }
