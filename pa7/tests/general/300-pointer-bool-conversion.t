bool init(int *p) {
  bool b = p;
  return b;
}

bool casted(int *p) {
  return static_cast<bool>(p);
}

int main() {
  int x = 0;
  int *p = &x;
  int *q = nullptr;
  return !init(p) || init(q) || !casted(p) || casted(q);
}
