int constructed = 0;
int destroyed = 0;

struct Element {
  int value;

  Element() noexcept : value(7) {
    constructed = constructed + 1;
  }

  ~Element() noexcept {
    destroyed = destroyed + 1;
  }
};

struct Holder {
  Element elements[3];
};

int make_count()
{
  Holder holder;
  return constructed - destroyed + holder.elements[2].value;
}

int main()
{
  int during = make_count();
  return during == 10 && constructed == 3 && destroyed == 3 ? 0 : 1;
}
