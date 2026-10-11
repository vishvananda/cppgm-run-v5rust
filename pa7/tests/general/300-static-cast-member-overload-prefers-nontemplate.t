struct V {};
struct A {
  void f(V*) {}
  template<class T> void f(T) {}
};
int main() {
  typedef void (A::*pointer)(V*);
  pointer selected = static_cast<pointer>(&A::f);
  (void)selected;
}
