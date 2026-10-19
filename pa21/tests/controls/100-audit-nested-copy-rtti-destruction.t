// Nested reference capture projects the outer polymorphic copy. An inert
// containing closure destructor remains empty in the frozen lowering; expect exit 0
namespace std {
class type_info { public: bool operator==(const type_info&) const; };
}
struct B { virtual ~B() {} };
struct D : B {};
int query(B& source) {
  return [=] { return [&] { return typeid(source) == typeid(D); }(); }();
}
int main() { B base; D derived; return query(base) || query(derived); }
