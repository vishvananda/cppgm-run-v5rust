// N3485 5.2.5p4, 3.9.3p3, 1.9p8: a member of a volatile object is volatile,
// so every read and write through a volatile object, pointer or reference
// is a volatile access; the plain object beside it stays ordinary.
struct Inner { int value; };
struct Device { int status; int data[2]; Inner inner; };

volatile Device global_device;

int read_reference(volatile Device& device) { return device.status; }
void write_pointer(volatile Device* device) { device->status = 5; }
int read_global() { return global_device.data[1]; }
int read_nested(volatile Device& device) { return device.inner.value; }
int read_plain(Device& device) { return device.status; }

int read_local()
{
  volatile Device local;
  local.data[0] = 6;
  local.inner.value = 4;
  return local.data[0] + local.inner.value;
}

int main()
{
  Device plain = {7, {8, 9}, {10}};
  write_pointer(&global_device);
  global_device.data[1] = 11;
  global_device.inner.value = 12;
  return read_reference(global_device) == 5 && read_global() == 11 &&
    read_nested(global_device) == 12 && read_plain(plain) == 7 &&
    read_local() == 10 ? 0 : 1;
}
