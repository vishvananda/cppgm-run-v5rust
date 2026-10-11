struct P { virtual ~P(); virtual int f(); };
struct VV : virtual P { ~VV(); };
int check();
