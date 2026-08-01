unit globals;
interface
type
  TFPUException = (exInvalidOp, exDenormalized, exZeroDivide,
                   exOverflow, exUnderflow, exPrecision);
  TFPUExceptionMask = set of TFPUException;
procedure SetFPUExceptionMask(const Mask: TFPUExceptionMask);
implementation
end.
