struct Base {
  int value;
  int implicit() const { return value; }
  int explicit_view() const { return this->value; }
  int repeated() const { return this->value + this->value; }
};
struct Derived : Base {
  int read() const { return this->explicit_view() + this->implicit(); }
};
int main() {
  Derived object;
  object.value = 7;
  return object.read() == object.repeated() ? 0 : 1;
}
