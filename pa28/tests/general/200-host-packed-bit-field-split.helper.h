// A packed bit-field spanning bytes no single access within its class
// holds lies where g++ puts it and is read and written piecewise: values
// written on either side read back on the other.
#pragma pack(push, 1)
struct Packed { short low : 3; int wide : 30; char after; };
struct Tail { char first; long value : 60; char last; };
#pragma pack(pop)

int host_read_wide(const Packed& value);
void host_write_wide(Packed* value, int wide);
long host_read_value(const Tail& value);
void host_write_value(Tail* value, long wide);
