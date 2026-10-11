struct Base {
  long w;

  Base();
  long width() const;
};

struct Pad {
  long p;

  Pad();
};

struct Mid : Pad, Base {
};

struct B : virtual Mid {
};

B & get_b();
