// N3485 3.9.3p3, 8.5.1p2, 8.5.1p7: every subobject of a volatile object is
// volatile, so each member and element that braced initialization of a
// volatile aggregate initializes, explicitly or by value-initialization of
// an omitted initializer, is written by a volatile access; a bit-field of a
// volatile object is too, when initialized and when assigned or incremented
// through it.
struct Inner { int value; };
struct Device { int status; int data[2]; Inner inner; unsigned bits : 3; int* next; };

int local()
{
  volatile Device device = {1, {2, 3}, {4}, 5, 0};
  return device.status + device.data[1] + device.inner.value + device.bits;
}

int partial()
{
  volatile Device device = {6};
  return device.data[0] + device.inner.value + device.bits;
}

int grid()
{
  volatile Device devices[2] = {{7, {8, 9}, {10}, 1, 0}, {11}};
  return devices[0].data[1] + devices[1].status;
}

int bits(volatile Device& device)
{
  device.bits = 2;
  ++device.bits;
  return device.bits;
}

int main()
{
  Device plain = {0, {0, 0}, {0}, 0, 0};
  return local() == 13 && partial() == 0 && grid() == 20 &&
    bits(plain) == 3 ? 0 : 1;
}
