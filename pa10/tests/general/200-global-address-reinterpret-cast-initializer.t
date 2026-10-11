typedef long intptr_type;

void target()
{
}

int object;
const void *function_address =
    reinterpret_cast<const void *>(reinterpret_cast<intptr_type>(&target));
const void *object_address = &object;

int main()
{
  return function_address != 0 && object_address == &object ? 0 : 1;
}
