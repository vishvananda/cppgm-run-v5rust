typedef __PTRDIFF_TYPE__ host_ptrdiff_t;
typedef __INTPTR_TYPE__ host_intptr_t;
typedef __SIZE_TYPE__ host_size_t;
typedef __UINTPTR_TYPE__ host_uintptr_t;

struct HostSizes {
  host_ptrdiff_t delta;
  host_intptr_t signed_ptr;
  host_size_t size;
  host_uintptr_t unsigned_ptr;
};

host_ptrdiff_t convert(host_size_t value) {
  return (host_ptrdiff_t)value;
}

host_uintptr_t widen(host_intptr_t value) {
  return (host_uintptr_t)value;
}

int main() {
  HostSizes sizes = { convert((host_size_t)0), (host_intptr_t)0, (host_size_t)0,
                      widen((host_intptr_t)0) };
  return (int)sizes.delta;
}
