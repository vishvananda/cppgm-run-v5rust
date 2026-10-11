// AUDIT-ID: LOOKUP-TAG
// AUDIT-EXPECT: reject
struct base { typedef int type; };
struct owner { class base; base::type invalid; };
