// AUDIT-ID: LOOKUP-TAG
// AUDIT-EXPECT: reject
struct owner { friend class invisible; };
invisible* invalid;
