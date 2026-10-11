struct Owner
{
  explicit Owner(int * ptr) : ptr(ptr) {}
  ~Owner() {}
  Owner(Owner && other) : ptr(other.ptr) { other.ptr = 0; }
  Owner(const Owner &) = delete;
  int * ptr;
};

Owner && as_rvalue(Owner & owner)
{
  return static_cast<Owner &&>(owner);
}

int consume(Owner && owner)
{
  return owner.ptr ? *owner.ptr : 0;
}

int main()
{
  int value = 7;
  Owner source(&value);
  int result = consume(as_rvalue(source));
  return result == 7 && source.ptr == &value ? 0 : 1;
}

// VALIDATION: compile-pass
