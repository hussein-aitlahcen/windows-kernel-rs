#define _AMD64_

#include "wdm.h"

NTSTATUS SafeMmProbeAndLockPages(
    PMDL MemoryDescriptorList,
    KPROCESSOR_MODE AccessMode,
    LOCK_OPERATION Operation
)
{
    NTSTATUS Status = STATUS_SUCCESS;
    __try
    {
        MmProbeAndLockPages(MemoryDescriptorList, AccessMode, Operation);
    }
    __except (EXCEPTION_EXECUTE_HANDLER)
    {
        Status = STATUS_ACCESS_VIOLATION;
    }
    return Status;
}