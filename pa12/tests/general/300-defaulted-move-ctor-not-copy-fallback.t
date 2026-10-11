struct Member {
  int value;

  Member() {
    value = 0;
  }

  Member(const Member& other) {
    value = other.value + 1000;
  }

  Member(Member&& other) {
    value = other.value;
    other.value = -1;
  }
};

struct X {
  Member member;

  X() {}

  X(const X& other) {
    member.value = other.member.value + 100;
  }

  X(X&& other) = default;
};

int main() {
  X a;
  a.member.value = 7;
  X b(static_cast<X&&>(a));
  return b.member.value == 7 && a.member.value == -1 ? 0 : 1;
}
