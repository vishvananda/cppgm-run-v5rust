// A compiler builtin is a global-namespace name, so the qualified spelling
// names the same one.  A hosted library writes it that way so a user's macro
// cannot intercept the call.
float qualified(float x, float y) { return ::__builtin_copysignf(x, y); }
double unqualified(double x) { return __builtin_fabs(x); }
double both(double x) { return ::__builtin_sqrt(x) + __builtin_sqrt(x); }

int main() { return 0; }
