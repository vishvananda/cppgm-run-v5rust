class YA { public: int x; };
class YB : virtual public YA {};
class YC : virtual public YA { public: int guard; };
class YD : public YB, public YC {};
int read(YB& b) { return b.x; }
int main() {
  YD d;
  d.guard = 13;
  d.x = 5;
  return d.x == 5 && read(d) == 5 ? 0 : 1;
}
