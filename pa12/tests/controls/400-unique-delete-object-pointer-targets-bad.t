// N3485 5.3.5p1: distinct object-pointer conversion declarations do not
// become unique by ranking their implicit object sequences; expect reject
struct Owner {
  operator int*();
  operator long*() const;
};
void release(Owner& owner) { delete owner; }
