struct D {
  D(int);
};

struct Holder {
  D d;
};

int main() {
  Holder h;
  (void)h;
  return 0;
}
