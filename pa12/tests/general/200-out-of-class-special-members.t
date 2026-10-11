class Y {
public:
  int y;

  Y();
  ~Y();
};

Y::Y() : y(4) {}
Y::~Y() {}

int main() {
  Y a;
  return a.y;
}
