// N3485 5.17p7, 5.7p5, 4.12: b += p is b = b + p (a pointer) converted to
// bool, so b becomes exactly true (1), not the pointer's low byte;
// expect exit 0
int arr[64];
int main() { bool b = false; b += arr + 1; return (int)b == 1 ? 0 : 1; }
