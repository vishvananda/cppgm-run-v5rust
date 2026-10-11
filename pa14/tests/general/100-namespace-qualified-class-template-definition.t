namespace outer {
namespace v {
template<class T> class C;
}

template<class T>
class v::C {
public:
  int f() const { return 7; }
};
}

int main() {
  outer::v::C<int> c;
  return c.f();
}
