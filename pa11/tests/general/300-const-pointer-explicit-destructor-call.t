int destroyed;

struct value {
  ~value() { destroyed = destroyed + 3; }
};

void destroy_const(const value *p) {
  p->~value();
}

int main() {
  return 0;
}
