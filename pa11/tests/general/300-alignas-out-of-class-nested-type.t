// VALIDATION: compile-pass

struct key_value_pair
{
  double value;
};

struct object
{
  struct table;
};

struct alignas(key_value_pair) object::table
{
  char size;
};

int main()
{
  if(alignof(object::table) != alignof(key_value_pair)) {
    return 1;
  }
  return sizeof(object::table) == alignof(key_value_pair) ? 0 : 1;
}
