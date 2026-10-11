struct S {
  int x;
};

const S& first(const S* p) {
  return p[0];
}

int main() {
  S item;
  item.x = 7;
  return first(&item).x == 7 ? 0 : 1;
}
