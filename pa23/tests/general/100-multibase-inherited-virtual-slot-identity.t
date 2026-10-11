struct left { virtual int first() = 0; };
struct right { virtual int second() = 0; };
struct interface : left, right {};
struct implementation : interface {
  int first() override { return 3; }
  int second() override { return 4; }
};
int main() { implementation value; interface& view = value;
  return view.first() != 3 || view.second() != 4; }
