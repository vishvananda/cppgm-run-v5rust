// AUDIT-ID: REF-BITFIELD
// AUDIT-EXPECT: reject
struct S{unsigned x:2;};int main(){S s;s.x=1;const volatile unsigned& r=s.x;return r;}
