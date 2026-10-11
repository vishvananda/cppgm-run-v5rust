// N3485 focus: 7.2 [dcl.enum] unscoped enumerations
enum EY { A, B = 4, C };
EY e;
int arr[C];
static_assert(C == 5, "bad");
