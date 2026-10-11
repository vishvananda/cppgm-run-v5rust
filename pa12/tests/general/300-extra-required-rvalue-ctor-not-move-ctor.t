struct Tag {
  int marker;
};

struct X {
  int value;

  X() {
    value = 0;
  }

  X(X&& other, int extra, Tag tag) {
    value = other.value + extra + tag.marker;
    other.value = -1;
  }
};

int main() {
  X a;
  a.value = 7;
  X b(static_cast<X&&>(a));
  return b.value == 7 && a.value == 7 ? 0 : 1;
}
