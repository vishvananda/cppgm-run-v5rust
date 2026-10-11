// N3485 2.14.4p1: a floating literal whose value is subnormal (1e-40f,
// 4.9406564584124654e-324) is a valid constant initializer; expect exit 0
double least = 4.9406564584124654e-324;
float small = 1e-40f;
int main() { return (least > 0 && small > 0) ? 0 : 1; }
