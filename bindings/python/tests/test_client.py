"""Tests for TrustLink Python client"""

import pytest
from unittest.mock import Mock, patch
from trustlink import TrustLinkClient, AuditEntry, AuditAction, Error


class TestTrustLinkClient:
    """Test cases for TrustLinkClient"""
    
    def setup_method(self):
        """Set up test client"""
        self.client = TrustLinkClient(
            contract_id="CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQAHHAGK6Z6E",
            rpc_url="https://soroban-testnet.stellar.org:443"
        )
    
    @patch('trustlink.client.TrustLinkClient._simulate_call')
    def test_get_audit_log_success(self, mock_simulate):
        """Test successful audit log retrieval"""
        # Mock response data
        mock_simulate.return_value = [
            {
                "attestation_id": "att_123",
                "action": "Created",
                "timestamp": 1640995200,
                "actor": "GDXLKEY5TR4IDEVSTRYUNYY3DPXQKQNSTDJ7HIVNFTJYQHOZXB7CRQME",
                "details": None
            },
            {
                "attestation_id": "att_123", 
                "action": "Renewed",
                "timestamp": 1672531200,
                "actor": "GDXLKEY5TR4IDEVSTRYUNYY3DPXQKQNSTDJ7HIVNFTJYQHOZXB7CRQME",
                "details": "Extended expiration by 365 days"
            }
        ]
        
        # Call method
        audit_log = self.client.get_audit_log("att_123")
        
        # Verify results
        assert len(audit_log) == 2
        
        # Check first entry
        assert audit_log[0].attestation_id == "att_123"
        assert audit_log[0].action == AuditAction.CREATED
        assert audit_log[0].timestamp == 1640995200
        assert audit_log[0].actor == "GDXLKEY5TR4IDEVSTRYUNYY3DPXQKQNSTDJ7HIVNFTJYQHOZXB7CRQME"
        assert audit_log[0].details is None
        
        # Check second entry
        assert audit_log[1].attestation_id == "att_123"
        assert audit_log[1].action == AuditAction.RENEWED
        assert audit_log[1].timestamp == 1672531200
        assert audit_log[1].details == "Extended expiration by 365 days"
        
        # Verify simulate_call was called correctly
        mock_simulate.assert_called_once_with("get_audit_log", [mock_simulate.return_value])
    
    @patch('trustlink.client.TrustLinkClient._simulate_call')
    def test_get_audit_log_empty(self, mock_simulate):
        """Test audit log retrieval for attestation with no entries"""
        mock_simulate.return_value = []
        
        audit_log = self.client.get_audit_log("att_456")
        
        assert len(audit_log) == 0
        assert isinstance(audit_log, list)
    
    @patch('trustlink.client.TrustLinkClient._simulate_call')
    def test_get_audit_log_error(self, mock_simulate):
        """Test audit log retrieval with contract error"""
        mock_simulate.side_effect = Exception("Contract not found")
        
        with pytest.raises(Error) as exc_info:
            self.client.get_audit_log("invalid_id")
        
        assert "Failed to get audit log" in str(exc_info.value)
    
    @patch('trustlink.client.TrustLinkClient._simulate_call')
    def test_get_audit_log_malformed_data(self, mock_simulate):
        """Test audit log with malformed response data"""
        mock_simulate.return_value = [
            {
                "action": "Created",
                "timestamp": 1640995200,
                # Missing required fields
            }
        ]
        
        audit_log = self.client.get_audit_log("att_789")
        
        # Should handle missing fields gracefully
        assert len(audit_log) == 1
        assert audit_log[0].action == AuditAction.CREATED
        assert audit_log[0].timestamp == 1640995200
        assert audit_log[0].attestation_id == "att_789"  # Should default to input
        assert audit_log[0].actor == ""  # Should default to empty string


if __name__ == "__main__":
    pytest.main([__file__])