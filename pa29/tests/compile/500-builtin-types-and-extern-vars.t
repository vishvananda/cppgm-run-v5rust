typedef __builtin_va_list my_va_list;

extern "C" {
  extern char* suboptarg;
  extern _Float16 __fabsf16(_Float16);
}

struct Bits {
  unsigned a : 1;
  unsigned b : 2;
};

union wait_t {
  int w_status;
  struct {
    unsigned a : 1;
    unsigned b : 2;
  } w_T;
};

int main() {
  return 0;
}
