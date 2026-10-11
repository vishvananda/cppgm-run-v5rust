#include "200-host-destructor-entry-emission.helper.h"

int Base::key() { return 1; }
int Derived::key() { return 2; }
int VDerived::key() { return 3; }
Derived* host_make_derived() { return new Derived; }
VDerived* host_make_vderived() { return new VDerived; }

HBase::~HBase() { note_destroyed(10); }
HSecond::~HSecond() { note_destroyed(20); }
HDerived::~HDerived() { note_destroyed(30); }
HVBase::~HVBase() { note_destroyed(40); }
HVDerived::~HVDerived() { note_destroyed(50); }
void host_delete_second(HSecond* value) { delete value; }
void host_delete_vbase(HVBase* value) { delete value; }
