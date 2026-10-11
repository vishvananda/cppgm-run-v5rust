struct BaseField {};

struct Base {
  BaseField *seen;

  explicit Base(BaseField *field) : seen(field) {}

  int ok(BaseField *field) const {
    return seen == field;
  }
};

typedef Base Alias;

struct Derived : Alias {
  BaseField field;

  Derived() : Alias(&field) {}
};

int main() {
  Derived d;
  return d.ok(&d.field) ? 0 : 1;
}
