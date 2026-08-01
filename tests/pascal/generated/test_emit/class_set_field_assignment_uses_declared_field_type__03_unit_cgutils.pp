unit cgutils;
interface
uses cpubase;
const
  tmpmaxcpufpuintreg = first_int_imreg +
    ((first_fpu_imreg - first_int_imreg) *
     ord(first_int_imreg < first_fpu_imreg));
  maxcpuregister = (tmpmaxcpufpuintreg +
    ((first_mm_imreg - tmpmaxcpufpuintreg) *
     ord(tmpmaxcpufpuintreg < first_mm_imreg)))-1;
type
  tcpuregisterset = set of 0..maxcpuregister;
implementation
end.
