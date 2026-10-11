constexpr int status = 0x17f;
#define W_INT(value) (*(int *)&(value))

static_assert((W_INT(status) & 0177) == 0177 &&
              (W_INT(status) >> 8) != 0x13,
              "stopped");

int main() { return 0; }
