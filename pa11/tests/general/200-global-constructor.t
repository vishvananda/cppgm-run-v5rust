int gx;

struct YP {
  YP() { gx = 5; }
  ~YP() { gx = gx + 1; }
};

YP g;

int main() {
  return gx;
}
