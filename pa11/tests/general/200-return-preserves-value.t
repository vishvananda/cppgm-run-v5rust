int gx;

struct YP {
  ~YP() { gx = 9; }
};

int main() {
  YP p;
  return 3;
}
