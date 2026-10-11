// VALIDATION: compile-pass
// An explicit pseudo-destructor call for a trivially destructible class is a no-op.

struct Item
{
  unsigned value;
};

void destroy(Item *pointer)
{
  pointer->~Item();
}

int main()
{
  Item item = {7};
  destroy(&item);
  return item.value == 7 ? 0 : 1;
}
