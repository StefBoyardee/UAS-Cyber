/* -*- Mode:C++; c-file-style:"gnu"; indent-tabs-mode:nil; -*- */
/*
 * Copyright (c) 2005,2006,2007 INRIA
 * Copyright (c) 2020 Orange Labs
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License version 2 as
 * published by the Free Software Foundation;
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 59 Temple Place, Suite 330, Boston, MA  02111-1307  USA
 *
 * Authors: Mathieu Lacage <mathieu.lacage@sophia.inria.fr>
 *          Rediet <getachew.redieteab@orange.com>
 */

#ifndef WIFI_PHY_COMMON_H
#define WIFI_PHY_COMMON_H

#include <ostream>
#include "ns3/fatal-error.h"

/**
 * \file
 * \ingroup wifi
 * Declaration of the following enums:
 * - ns3::WifiPreamble
 * - ns3::WifiModulationClass
 * - ns3::WifiPpduField
 * - ns3::WifiPpduType
 * - ns3::WifiPhyRxfailureReason
 */

namespace ns3 {

/**
 * These constants define the various convolutional coding rates
 * used for the OFDM transmission modes in the IEEE 802.11
 * standard. DSSS (for example) rates which do not have an explicit
 * coding stage in their generation should have this parameter set to
 * WIFI_CODE_RATE_UNDEFINED.
 * \note This typedef and constants could be converted to an enum or scoped
 * enum if pybindgen is upgraded to support Callback<WifiCodeRate>
 */
typedef uint16_t WifiCodeRate;
const uint16_t WIFI_CODE_RATE_UNDEFINED = 0;
const uint16_t WIFI_CODE_RATE_1_2 = 1;
const uint16_t WIFI_CODE_RATE_2_3 = 2;
const uint16_t WIFI_CODE_RATE_3_4 = 3;
const uint16_t WIFI_CODE_RATE_5_6 = 4;

/**
 * \ingroup wifi
 * The type of preamble to be used by an IEEE 802.11 transmission
 */
enum WifiPreamble
{
  WIFI_PREAMBLE_LONG,
  WIFI_PREAMBLE_SHORT,
  WIFI_PREAMBLE_HT_MF,
  WIFI_PREAMBLE_VHT_SU,
  WIFI_PREAMBLE_VHT_MU,
  WIFI_PREAMBLE_HE_SU,
  WIFI_PREAMBLE_HE_ER_SU,
  WIFI_PREAMBLE_HE_MU,
  WIFI_PREAMBLE_HE_TB
};

/**
 * \brief Stream insertion operator.
 *
 * \param os the stream
 * \param preamble the preamble
 * \returns a reference to the stream
 */
inline std::ostream& operator<< (std::ostream &os, const WifiPreamble &preamble)
{
  switch (preamble)
    {
      case WIFI_PREAMBLE_LONG:
        return (os << "LONG");
      case WIFI_PREAMBLE_SHORT:
        return (os << "SHORT");
      case WIFI_PREAMBLE_HT_MF:
        return (os << "HT_MF");
      case WIFI_PREAMBLE_VHT_SU:
        return (os << "VHT_SU");
      case WIFI_PREAMBLE_VHT_MU:
        return (os << "VHT_MU");
      case WIFI_PREAMBLE_HE_SU:
        return (os << "HE_SU");
      case WIFI_PREAMBLE_HE_ER_SU:
        return (os << "HE_ER_SU");
      case WIFI_PREAMBLE_HE_MU:
        return (os << "HE_MU");
      case WIFI_PREAMBLE_HE_TB:
        return (os << "HE_TB");
      default:
        NS_FATAL_ERROR ("Invalid preamble");
        return (os << "INVALID");
    }
}

/**
 * \ingroup wifi
 * This enumeration defines the modulation classes per
 * (Table 10-6 "Modulation classes"; IEEE 802.11-2016, with
 * updated in 802.11ax/D6.0 as Table 10-9).
 */
enum WifiModulationClass
{
  /** Modulation class unknown or unspecified. A WifiMode with this
  WifiModulationClass has not been properly initialized. */
  WIFI_MOD_CLASS_UNKNOWN = 0,
  WIFI_MOD_CLASS_DSSS,     //!< DSSS (Clause 15)
  WIFI_MOD_CLASS_HR_DSSS,  //!< HR/DSSS (Clause 16)
  WIFI_MOD_CLASS_ERP_OFDM, //!< ERP-OFDM (18.4)
  WIFI_MOD_CLASS_OFDM,     //!< OFDM (Clause 17)
  WIFI_MOD_CLASS_HT,       //!< HT (Clause 19)
  WIFI_MOD_CLASS_VHT,      //!< VHT (Clause 22)
  WIFI_MOD_CLASS_HE        //!< HE (Clause 27)
};

/**
 * \brief Stream insertion operator.
 *
 * \param os the stream
 * \param modulation the WifiModulationClass
 * \returns a reference to the stream
 */
inline std::ostream& operator<< (std::ostream &os, const WifiModulationClass &modulation)
{
  switch (modulation)
    {
      case WIFI_MOD_CLASS_DSSS:
        return (os << "DSSS");
      case WIFI_MOD_CLASS_HR_DSSS:
        return (os << "HR/DSSS");
      case WIFI_MOD_CLASS_ERP_OFDM:
        return (os << "ERP-OFDM");
      case WIFI_MOD_CLASS_OFDM:
        return (os << "OFDM");
      case WIFI_MOD_CLASS_HT:
        return (os << "HT");
      case WIFI_MOD_CLASS_VHT:
        return (os << "VHT");
      case WIFI_MOD_CLASS_HE:
        return (os << "HE");
      default:
        NS_FATAL_ERROR ("Unknown modulation");
        return (os << "unknown");
    }
}

/**
 * \ingroup wifi
 * The type of PPDU field (grouped for convenience)
 */
enum WifiPpduField
{
  /**
   * SYNC + SFD fields for DSSS or ERP,
   * shortSYNC + shortSFD fields for HR/DSSS or ERP,
   * HT-GF-STF + HT-GF-LTF1 fields for HT-GF,
   * L-STF + L-LTF fields otherwise.
   */
  WIFI_PPDU_FIELD_PREAMBLE = 0,
  /**
   * PHY header field for DSSS or ERP,
   * short PHY header field for HR/DSSS or ERP,
   * field not present for HT-GF,
   * L-SIG field or L-SIG + RL-SIG fields otherwise.
   */
  WIFI_PPDU_FIELD_NON_HT_HEADER,
  WIFI_PPDU_FIELD_HT_SIG,   //!< HT-SIG field
  WIFI_PPDU_FIELD_TRAINING, //!< STF + LTF fields (excluding those in preamble for HT-GF)
  WIFI_PPDU_FIELD_SIG_A,    //!< SIG-A field
  WIFI_PPDU_FIELD_SIG_B,    //!< SIG-B field
  WIFI_PPDU_FIELD_DATA      //!< data field
};

/**
 * \brief Stream insertion operator.
 *
 * \param os the stream
 * \param field the PPDU field
 * \returns a reference to the stream
 */
inline std::ostream& operator<< (std::ostream &os, const WifiPpduField &field)
{
  switch (field)
    {
      case WIFI_PPDU_FIELD_PREAMBLE:
        return (os << "preamble");
      case WIFI_PPDU_FIELD_NON_HT_HEADER:
        return (os << "non-HT header");
      case WIFI_PPDU_FIELD_HT_SIG:
        return (os << "HT-SIG");
      case WIFI_PPDU_FIELD_TRAINING:
        return (os << "training");
      case WIFI_PPDU_FIELD_SIG_A:
        return (os << "SIG-A");
      case WIFI_PPDU_FIELD_SIG_B:
        return (os << "SIG-B");
      case WIFI_PPDU_FIELD_DATA:
        return (os << "data");
      default:
        NS_FATAL_ERROR ("Unknown field");
        return (os << "unknown");
    }
}

/**
 * \ingroup wifi
 * The type of PPDU (SU, DL MU, or UL MU)
 */
enum WifiPpduType
{
  WIFI_PPDU_TYPE_SU = 0,
  WIFI_PPDU_TYPE_DL_MU,
  WIFI_PPDU_TYPE_UL_MU
};

/**
 * \brief Stream insertion operator.
 *
 * \param os the stream
 * \param type the PPDU type
 * \returns a reference to the stream
 */
inline std::ostream& operator<< (std::ostream &os, const WifiPpduType &type)
{
  switch (type)
    {
      case WIFI_PPDU_TYPE_SU:
        return (os << "SU");
      case WIFI_PPDU_TYPE_DL_MU:
        return (os << "DL MU");
      case WIFI_PPDU_TYPE_UL_MU:
        return (os << "UL MU");
      default:
        NS_FATAL_ERROR ("Unknown type");
        return (os << "unknown");
    }
}

/**
 * \ingroup wifi
 * Enumeration of the possible reception failure reasons.
 */
enum WifiPhyRxfailureReason
{
  UNKNOWN = 0,
  UNSUPPORTED_SETTINGS,
  CHANNEL_SWITCHING,
  RXING,
  TXING,
  SLEEPING,
  BUSY_DECODING_PREAMBLE,
  PREAMBLE_DETECT_FAILURE,
  RECEPTION_ABORTED_BY_TX,
  L_SIG_FAILURE,
  HT_SIG_FAILURE,
  SIG_A_FAILURE,
  SIG_B_FAILURE,
  PREAMBLE_DETECTION_PACKET_SWITCH,
  FRAME_CAPTURE_PACKET_SWITCH,
  OBSS_PD_CCA_RESET,
  HE_TB_PPDU_TOO_LATE,
  FILTERED
};

/**
 * \brief Stream insertion operator.
 *
 * \param os the stream
 * \param reason the failure reason
 * \returns a reference to the stream
 */
inline std::ostream& operator<< (std::ostream &os, const WifiPhyRxfailureReason &reason)
{
  switch (reason)
    {
      case UNSUPPORTED_SETTINGS:
        return (os << "UNSUPPORTED_SETTINGS");
      case CHANNEL_SWITCHING:
        return (os << "CHANNEL_SWITCHING");
      case RXING:
        return (os << "RXING");
      case TXING:
        return (os << "TXING");
      case SLEEPING:
        return (os << "SLEEPING");
      case BUSY_DECODING_PREAMBLE:
        return (os << "BUSY_DECODING_PREAMBLE");
      case PREAMBLE_DETECT_FAILURE:
        return (os << "PREAMBLE_DETECT_FAILURE");
      case RECEPTION_ABORTED_BY_TX:
        return (os << "RECEPTION_ABORTED_BY_TX");
      case L_SIG_FAILURE:
        return (os << "L_SIG_FAILURE");
      case HT_SIG_FAILURE:
        return (os << "HT_SIG_FAILURE");
      case SIG_A_FAILURE:
        return (os << "SIG_A_FAILURE");
      case SIG_B_FAILURE:
        return (os << "SIG_B_FAILURE");
      case PREAMBLE_DETECTION_PACKET_SWITCH:
        return (os << "PREAMBLE_DETECTION_PACKET_SWITCH");
      case FRAME_CAPTURE_PACKET_SWITCH:
        return (os << "FRAME_CAPTURE_PACKET_SWITCH");
      case OBSS_PD_CCA_RESET:
        return (os << "OBSS_PD_CCA_RESET");
      case HE_TB_PPDU_TOO_LATE:
        return (os << "HE_TB_PPDU_TOO_LATE");
      case FILTERED:
        return (os << "FILTERED");
      case UNKNOWN:
      default:
        NS_FATAL_ERROR ("Unknown reason");
        return (os << "UNKNOWN");
    }
}

} //namespace ns3

#endif /* WIFI_PHY_COMMON_H */
