// A non-variadic consumer copies a va_list into local storage before reading
// a larger aggregate. Frame ordering uses the typed MIR operand footprint.
struct Big { long x[4]; };
long take(__builtin_va_list original) {
 __builtin_va_list copy;
 __builtin_va_copy(copy,original);
 Big result=__builtin_va_arg(copy,Big);
 long answer=result.x[0]+result.x[3];
 __builtin_va_end(copy);
 return answer;
}
long source(int n,...) { __builtin_va_list ap;__builtin_va_start(ap,n);long v=take(ap);__builtin_va_end(ap);return v;}
int main() {Big b={17,3,4,5};return source(1,b)!=22;}
