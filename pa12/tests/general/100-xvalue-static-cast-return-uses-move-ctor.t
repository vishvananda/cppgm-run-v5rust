struct Owner
{
  Owner() : tag(0) {}
  Owner(Owner &&) : tag(1) {}
  Owner(const Owner &) : tag(2) {}
  int tag;
};

Owner make(Owner & owner)
{
  return static_cast<Owner &&>(owner);
}

int main()
{
  Owner source;
  Owner result = make(source);
  return result.tag == 1 ? 0 : 1;
}

// VALIDATION: compile-pass
