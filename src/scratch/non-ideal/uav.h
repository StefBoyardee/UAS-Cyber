#pragma once

#include "ns3/udp-echo-helper.h"

#include <stdint.h>
#include "ns3/application-container.h"
#include "ns3/application.h"
#include "ns3/node-container.h"
#include "ns3/object-factory.h"
#include "ns3/event-id.h"
#include "ns3/ipv4-address.h"
#include "ns3/ipv6-address.h"
#include "ns3/uinteger.h"
#include "ns3/ptr.h"
#include "ns3/address.h"
#include "ns3/traced-callback.h"
#include "ns3/vector.h"

using namespace ns3;
namespace ns3 {

  class Socket;

}

using UAVDataType_ = uint8_t;
namespace UAVDataType
{
	constexpr UAVDataType_ VIRTUAL_FORCES_POSITION = 0;
	constexpr UAVDataType_ VIRTUAL_FORCES_CENTRAL_POSITION = 1;
};

struct UAVData
{
	Vector position;//Normal SI units
	UAVDataType_ type;
};


struct SwarmEntry
{
  UAVData data;
};

class UAV : public Application
{
public:
  /**
   * \brief Get the type ID.
   * \return the object TypeId
   */
  static TypeId GetTypeId (void);
  UAV ();
  virtual ~UAV ();

  virtual UAVDataType_ GetUAVType() { return UAVDataType::VIRTUAL_FORCES_CENTRAL_POSITION; }

protected:
  virtual void DoDispose (void);

  void BroadcastPosition();
  void Send();
  void Cyberattack();

  void Calculate();

private:
  virtual void StartApplication (void);
  virtual void StopApplication (void);

  /**
   * \brief Handle a packet reception.
   *
   * This function is called by lower layers.
   *
   * \param socket the socket the packet was received to.
   */
  void HandleRead (Ptr<Socket> socket);

  UAVDataType_ m_uavType;
  Ipv4Address m_uavAddress;
  Time m_packetInterval;
  Time m_calculateInterval;
  uint32_t m_uavCount;
  Ipv4Address m_rootAddress;

  std::map<Ipv4Address, int> m_packetRecvCount;
  std::map<Ipv4Address, int> m_packetSendCount;

  Vector m_velocity = {};

  uint32_t m_sent;
  EventId m_sendEvent, m_calculateEvent;
  
  uint16_t m_port; //!< Port on which we listen for incoming packets.
  Ptr<Socket> m_socket; //!< IPv4 Socket
  Ipv4Address m_local; //!< local multicast address

  std::map<Ipv4Address, SwarmEntry> m_swarmData;

  /// Callbacks for tracing the packet Rx events
  TracedCallback<Ptr<const Packet>> m_rxTrace;

  /// Callbacks for tracing the packet Rx events, includes source and destination addresses
  TracedCallback<Ptr<const Packet>, const Address &, const Address &> m_rxTraceWithAddresses;
};



//============================== HELPERS ==============================


/**
 * \brief Create a server application which waits for input UDP packets
 *        and sends them back to the original sender.
 */
class UAVHelper : public Application
{

public:
  /**
   * Create UAVServerHelper which will make life easier for people trying
   * to set up simulations with echos.
   *
   * \param port The port the server will wait on for incoming packets
   */
  UAVHelper (Ipv4Address serverAddress, uint16_t port, UAVDataType_ type, Time interPacketInterval, Time calculateInterval, uint32_t uavCount);

  /**
   * Record an attribute to be set in each Application after it is is created.
   *
   * \param name the name of the attribute to set
   * \param value the value of the attribute to set
   */
  void SetAttribute (std::string name, const AttributeValue &value);

  /**
   * Create a UAVServerApplication on the specified Node.
   *
   * \param node The node on which to create the Application.  The node is
   *             specified by a Ptr<Node>.
   *
   * \returns An ApplicationContainer holding the Application created,
   */
  ApplicationContainer Install (Ptr<Node> node) const;

  /**
   * Create a UAVServerApplication on specified node
   *
   * \param nodeName The node on which to create the application.  The node
   *                 is specified by a node name previously registered with
   *                 the Object Name Service.
   *
   * \returns An ApplicationContainer holding the Application created.
   */
  ApplicationContainer Install (std::string nodeName) const;

  /**
   * \param c The nodes on which to create the Applications.  The nodes
   *          are specified by a NodeContainer.
   *
   * Create one udp echo server application on each of the Nodes in the
   * NodeContainer.
   *
   * \returns The applications created, one Application per Node in the 
   *          NodeContainer.
   */
  ApplicationContainer Install (NodeContainer c) const;

private:
  /**
   * Install an ns3::UAVServer on the node configured with all the
   * attributes set with SetAttribute.
   *
   * \param node The node on which an UAVServer will be installed.
   * \returns Ptr to the application installed.
   */
  Ptr<Application> InstallPriv (Ptr<Node> node) const;

  ObjectFactory m_factory; //!< Object factory.
};


