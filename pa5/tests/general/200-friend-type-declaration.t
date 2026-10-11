// HHC-136
struct X {
  friend struct Y;
  friend class Z;
};
