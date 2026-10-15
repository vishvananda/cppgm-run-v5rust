// expect reject
class Member {
  Member(int) {}
};
struct Holder {
  Member object;
  Holder() : object{3} {}
};
int main() { Holder object; }
