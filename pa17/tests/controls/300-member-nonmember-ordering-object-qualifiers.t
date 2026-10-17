// N3485 14.5.6.2p3: the inserted object retains cv/ref qualification.
// The selected member must not instantiate the unchosen free body; expect exit 0
struct A {
    template<class R> int operator*(R const&) const && { return 3; }
};
template<class T, class R> int operator*(T const&&, R const&) {
    return T::unchosen_body;
}
int main() {
    const int n = 0;
    return static_cast<A const&&>(A()) * n - 3;
}
