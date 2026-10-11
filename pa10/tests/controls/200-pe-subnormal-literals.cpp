// N3485 2.14.4p1: an in-range value rounds to the nearest representable value;
// subnormal literals are valid. expect exit 0
double g = 4.9e-324;
int main() { volatile float f = 1e-45f; volatile double d = 2.2250738585072011e-308; return (g > 0 && f > 0 && d > 0) ? 0 : 1; }
