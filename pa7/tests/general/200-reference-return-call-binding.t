int& id(int&);
void keep(int&);

void f(int& x) {
  keep(id(x));
}
