#include "wrapper_base.h"

#include "ntdef.h"

struct _DRIVE_LAYOUT_INFORMATION_EX;

union _KIDTENTRY64
{
    struct
    {
        USHORT OffsetLow;                                                   //0x0
        USHORT Selector;                                                    //0x2
    };
    USHORT IstIndex:3;                                                      //0x4
    USHORT Reserved0:5;                                                     //0x4
    USHORT Type:5;                                                          //0x4
    USHORT Dpl:2;                                                           //0x4
    struct
    {
        USHORT Present:1;                                                   //0x4
        USHORT OffsetMiddle;                                                //0x6
    };
    struct
    {
        ULONG OffsetHigh;                                                   //0x8
        ULONG Reserved1;                                                    //0xc
    };
    ULONGLONG Alignment;                                                    //0x0
}; 

union _KGDTENTRY64
{
    struct
    {
        USHORT LimitLow;                                                    //0x0
        USHORT BaseLow;                                                     //0x2
    };
    struct
    {
        UCHAR BaseMiddle;                                                   //0x4
        UCHAR Flags1;                                                       //0x5
        UCHAR Flags2;                                                       //0x6
        UCHAR BaseHigh;                                                     //0x7
    } Bytes;                                                                //0x4
    struct
    {
        struct
        {
            ULONG BaseMiddle:8;                                                 //0x4
            ULONG Type:5;                                                       //0x4
            ULONG Dpl:2;                                                        //0x4
            ULONG Present:1;                                                    //0x4
            ULONG LimitHigh:4;                                                  //0x4
            ULONG System:1;                                                     //0x4
            ULONG LongMode:1;                                                   //0x4
            ULONG DefaultBig:1;                                                 //0x4
            ULONG Granularity:1;                                                //0x4
            ULONG BaseHigh:8;                                                   //0x4
        } Bits;                                                                 //0x4
        ULONG BaseUpper;                                                    //0x8
    };
    struct
    {
        ULONG MustBeZero;                                                   //0xc
        LONGLONG DataLow;                                                   //0x0
    };
    LONGLONG DataHigh;                                                      //0x8
};

#include "ntifs.h"