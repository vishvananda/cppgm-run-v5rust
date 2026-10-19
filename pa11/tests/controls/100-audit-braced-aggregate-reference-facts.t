// expect exit 0
struct A { int n; };
struct B { A a; int n; };
int local() {
  const A& a = {1};
  const B& b = {A{2}, 3};
  return a.n + b.a.n + b.n;
}
int persistent() {
  static const A& a = {4};
  static const B& b = {A{5}, 6};
  return a.n + b.a.n + b.n;
}
int main() { return local() == 6 && persistent() == 15 && persistent() == 15 ? 0 : 1; }
