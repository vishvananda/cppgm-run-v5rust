struct record { long signature; char bytes[2]; };

extern record value;
int read() { return value.signature; }
int observed = read();
record value = { 7, { 0 } };

int main() { return observed != 7; }
