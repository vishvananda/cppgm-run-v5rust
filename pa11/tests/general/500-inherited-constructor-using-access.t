// An inheriting using-declaration does not replace the access
// of the selected base constructor.
struct base { base(int) {} };
class derived : public base { using base::base; };

int main() {
  derived value(1);
  return 0;
}
