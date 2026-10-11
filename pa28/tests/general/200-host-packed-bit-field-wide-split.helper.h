// A packed bit-field spanning more than 8 bytes lies where g++ puts it and
// is read and written piecewise: values written on either side of a g++
// object read back on the other.
#pragma pack(push, 1)
struct Spans { char low : 3; unsigned long long wide : 64; char after; };
#pragma pack(pop)

unsigned long long host_read(const Spans& value);
void host_write(Spans* value, unsigned long long wide);
