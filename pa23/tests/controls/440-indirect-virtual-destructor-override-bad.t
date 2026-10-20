// The implicit destructor includes an indirect virtual base's throwing
// specification and cannot override a nonthrowing direct base; expect reject
struct V { virtual ~V() noexcept(false) {} };
struct A : virtual V { ~A() noexcept {} };
struct D : A {};
int main() { return 0; }
