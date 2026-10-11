struct C {
  operator int() const;
};

C::operator int() const { return 0; }
